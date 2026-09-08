use std::io::{self, ErrorKind, Read};
use std::fs::{self, File};
use std::os::unix::thread::JoinHandleExt;
use std::process::Command;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use flate2::read::GzDecoder;
use tar::Archive;
use zbus::blocking::{Connection, Proxy};
use zbus::blocking::fdo::DBusProxy;

fn show_gui_error(err: &str) {
    rfd::MessageDialog::new()
        .set_title("ClkMonOff start error")
        .set_description(err)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

fn load_lkm(data: &[u8], params: &str, exists_ok: bool) -> io::Result<()> {
    let p = std::ffi::CString::new(params)?;
    if unsafe { libc::syscall(
        libc::SYS_init_module,
        data.as_ptr(),
        data.len() as libc::size_t,
        p.as_ptr(),
    )} == 0 { Ok(()) } else {
        let e = io::Error::last_os_error();
        if exists_ok && e.kind() == ErrorKind::AlreadyExists { return Ok(()); }
        Err(e)
    }
}

fn unload_lkm(name: &str) -> io::Result<()> {
    let p = std::ffi::CString::new(name)?;
    if unsafe { libc::syscall(libc::SYS_delete_module, p.as_ptr(), 0) } == 0 { Ok(()) } else {
        Err(io::Error::last_os_error())
    }
}

const SRC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/src.tar.gz"));
const SRC_HASH: &str = env!("SRC_HASH");

fn app_path() -> PathBuf {
    let home = std::env::var("SUDO_USER").map(|u| format!("/home/{}", u)).or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| "/root".into());
    let conf = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(home).join(".config"));
    conf.join("clkmonoff")
}

fn get_lkm() -> io::Result<Vec<u8>> {
    let kver = fs::read_to_string("/proc/sys/kernel/osrelease")?.trim().to_string();
    let dir = app_path().join("kmod").join(format!("{}-{}", kver, SRC_HASH));
    let ko = dir.join("clk_linux_km.ko");
    if !ko.exists() {
        let bdir = dir.join("build");
        fs::create_dir_all(&bdir)?;
        Archive::new(GzDecoder::new(SRC)).unpack(&bdir)?;
        let make = std::env::var("MAKE").unwrap_or_else(|_| "make".into());
        let out = Command::new(make).args(["-C"]).arg(&bdir).output()?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            eprintln!("Compile error:\n{}", err);
            show_gui_error(&err);
            let _ = fs::remove_dir_all(&bdir);
            return Err(io::Error::new(ErrorKind::Other, "module build failed"));
        }
        fs::rename(bdir.join("clk_linux_km.ko"), &ko)?;
        let _ = fs::remove_dir_all(&bdir);
    }
    fs::read(ko)
}

static THREAD_TID: AtomicUsize = AtomicUsize::new(0);
fn setup_signals() {
    extern "C" fn sig_handler(_: libc::c_int) {
        let tid = THREAD_TID.swap(0, Ordering::SeqCst);
        if tid != 0 {
            unsafe { libc::pthread_cancel(tid as libc::pthread_t); }
        }
    }
    for &sig in &[libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
        unsafe { libc::signal(sig, sig_handler as *const () as usize); }
    }
}

fn str_codes(v: Vec<u16>) -> String { v.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",") }
pub fn run(on: Vec<u16>, off: Vec<u16>) -> Result<(), Box<dyn std::error::Error>> {
    let lkm = get_lkm()?;
    let params = format!("keys_on=\"{}\" keys_off=\"{}\"", str_codes(on), str_codes(off));
    load_lkm(&lkm, &params, true)?;
    println!("LKM loaded");
    let lthread = thread::spawn(|| {
        let dbus_ctx = Connection::session().ok().and_then(|c| DBusProxy::new(&c).ok().map(|p| (c, p)));
        if dbus_ctx.is_none() { eprintln!("D-Bus session not found, media pause disabled"); }
        let mut file = match File::open("/dev/clkmonoff") {
            Ok(f) => f,
            Err(e) => { eprintln!("Failed to open device: {}", e); return; }
        };
        let mut buf = [0u8; 1];
        println!("Started");
        loop {
            if let Err(e) = file.read_exact(&mut buf) {
                eprintln!("{}", e);
                break;
            }
            println!("Changed");
            let act = buf[0] != 0;
            if act && let Some((conn, dbus)) = &dbus_ctx {
                let Ok(list) = dbus.list_names() else { continue };
                for name in list {
                    if !name.starts_with("org.mpris.MediaPlayer2.") { continue; }
                    let Ok(proxy) = Proxy::new(conn, name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player") else { continue };
                    let _: Result<(), _> = proxy.call("Pause", &());
                }
            }
        }
    });
    THREAD_TID.store(lthread.as_pthread_t() as usize, Ordering::SeqCst);
    setup_signals();

    let _ = lthread.join();
    if let Err(e) = unload_lkm("clk_linux_km") {
        eprintln!("Failed to unload LKM: {}", e);
    }
    Ok(())
}

pub fn clear() {
    let _ = unload_lkm("clk_linux_km");
    if let Err(e) = fs::remove_dir_all(app_path()) {
        if e.kind() == ErrorKind::NotFound {
            eprintln!("Already clean");
        } else {
            eprintln!("Error: {}", e);
        }
    } else {
        println!("Cleared");
    }
}
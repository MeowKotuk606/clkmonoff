use crate::*;
use std::sync::{Arc, Condvar, Mutex, mpsc, atomic::{AtomicBool, Ordering}, OnceLock};
use windows::core::{w, PCWSTR, BOOL, PWSTR};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as SessionManager;
use windows::Win32::{
    Foundation::*,
    System::{
        Com::*,
        Power::*,
        SystemServices::*,
        LibraryLoader::*
    },
    Graphics::Gdi::*,
    Devices::Display::*,
    Media::Audio::{*, Endpoints::*},
    UI::{
        WindowsAndMessaging::*,
        Accessibility::*
    }
};

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

const INT_MAX: i32 = 2147483647;
const HC_ACTION: i32 = 0;

unsafe fn ol_fix() {
    let state = global_state();
    let _ = SetWindowPos(state.overlay, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
}

static GLOBAL_STATE: OnceLock<GlobalState> = OnceLock::new();
fn global_state<'a>() -> &'a GlobalState { GLOBAL_STATE.get().unwrap() }
unsafe extern "system" fn kb_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let state = global_state();
    if code == HC_ACTION {
        let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let _ = state.kb_tx.send((kb.vkCode as usize, match w_param.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => { 1 }
            _ => { 0 }
        }));
        if state.act() { return LRESULT(1); }
    }
    CallNextHookEx(Some(HHOOK(null())), code, w_param, l_param)
}

unsafe extern "system" fn ms_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let state = global_state();
    if code == HC_ACTION && state.act() {
        SetCursor(None);
        return LRESULT(1);
    }
    CallNextHookEx(Some(HHOOK(null())), code, w_param, l_param)
}
unsafe extern "system" fn ol_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if let Some(state) = GLOBAL_STATE.get() && msg == WM_POWERBROADCAST && state.act() && w_param.0 == PBT_POWERSETTINGCHANGE as usize {
        let pbs = &*(l_param.0 as *const POWERBROADCAST_SETTING);
        if pbs.PowerSetting == GUID_CONSOLE_DISPLAY_STATE && (pbs.Data.as_ptr().cast::<u32>().read_unaligned() == 1)
            || pbs.PowerSetting == GUID_MONITOR_POWER_ON {
            let _ = SendNotifyMessageW(HWND_BROADCAST, WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(2));
            return LRESULT(1);
        }
    } else if msg == WM_DESTROY || msg == WM_CLOSE || msg == WM_CREATE {
        if msg == WM_DESTROY { PostQuitMessage(0); }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, w_param, l_param)
}

unsafe extern "system" fn we_proc(_: HWINEVENTHOOK, _: u32, hwnd: HWND, _: i32, _: i32, _: u32, _: u32) {
    let state = global_state();
    if state.act() && hwnd != state.overlay { ol_fix(); }
}

unsafe extern "system" fn mon_proc(mon: HMONITOR, _: HDC, _: *mut RECT, _: LPARAM) -> BOOL {
    let mut count: u32 = 0;
    if GetNumberOfPhysicalMonitorsFromHMONITOR(mon, &mut count).is_err() { return FALSE; }
    let mut mons = vec![unsafe { std::mem::zeroed::<PHYSICAL_MONITOR>() }; count as usize];
    if GetPhysicalMonitorsFromHMONITOR(mon, &mut mons).is_err() { return FALSE; }
    for mon in mons.iter() { SetVCPFeature(mon.hPhysicalMonitor, 0xD6, 0x05); }
    let _ = DestroyPhysicalMonitors(&mut mons);
    TRUE
}

const fn null<T>() -> *mut T { std::ptr::null_mut() }

#[derive(Debug)]
struct GlobalState {
    kb_tx: mpsc::Sender<(usize, i32)>,
    overlay: HWND,
    act: AtomicBool
}
impl GlobalState {
    fn act(&self) -> bool { self.act.load(Ordering::Relaxed) }
}
unsafe impl Send for GlobalState {}
unsafe impl Sync for GlobalState {}

unsafe fn find_show_win(clz: PCWSTR, cmd: SHOW_WINDOW_CMD) {
    if let Ok(w) = FindWindowW(clz, None) {
        let _ = ShowWindow(w, cmd);
    }
}

unsafe fn init(tx: mpsc::Sender<(usize, i32)>, cond: Arc<(Mutex<bool>, Condvar)>) -> Result<(), Box<dyn std::error::Error>> {
    let inst = HINSTANCE(GetModuleHandleW(None)?.0);
    let inst_raw = inst.0 as usize;
    std::thread::spawn(move || {
        let inst = HINSTANCE(inst_raw as *mut _);

        let mut wc = WNDCLASSEXW::default();
        wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(ol_proc);
        wc.hInstance = inst;
        wc.lpszClassName = w!("ClkMonOffOverlay");
        wc.hbrBackground = CreateSolidBrush(rgb(0, 0, 0));
        wc.hCursor = HCURSOR(null());
        RegisterClassExW(&wc);
        let ol = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            wc.lpszClassName, None, WS_POPUP,
            0, 0, INT_MAX, INT_MAX, None, None, Some(inst), None
        ).unwrap();
        SetLayeredWindowAttributes(ol, COLORREF(0), 255, LWA_ALPHA).unwrap();
        let _ = ShowWindow(ol, SW_HIDE);
        GLOBAL_STATE.set(GlobalState {
            kb_tx: tx,
            act: AtomicBool::new(false),
            overlay: ol
        }).unwrap();

        let cond0 = cond.clone();
        std::thread::spawn(|| {
            let cond = cond0;
            let Ok(_) = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() else {
                println!("[SMTC] Failed to initialize COM");
                return
            };
            let m = if let Ok(op) = SessionManager::RequestAsync() {
                let Ok(r) = op.join() else {
                    println!("[SMTC] Failed to initialize");
                    return
                };
                r
            } else {
                println!("[SMTC] Failed to initialize");
                return;
            };
            let mut g = cond.0.lock().unwrap();
            loop {
                g = cond.1.wait(g).unwrap();
                if *g {
                    let Ok(l) = m.GetSessions() else { continue };
                    let mut tl = Vec::new();
                    for s in l { if let Ok(a) = s.TryPauseAsync() { tl.push(a); } }
                    for op in tl { let _ = op.join(); }
                }
            }
        });

        let cond0 = cond.clone();
        std::thread::spawn(|| {
            let cond = cond0;
            let Ok(_) = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() else {
                println!("[I2C] Failed to initialize COM");
                return
            };
            let mut g = cond.0.lock().unwrap();
            let mut sv: HashMap<String, (usize, bool)> = HashMap::new();
            loop {
                g = cond.1.wait(g).unwrap();
                let Ok(e): Result<IMMDeviceEnumerator, _> = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) else { continue };
                if *g {
                    let Ok(l) = e.EnumAudioEndpoints(eAll, DEVICE_STATE_ACTIVE) else { continue };
                    let Ok(c) = l.GetCount() else { continue };
                    for i in 0..c {
                        let Ok(d) = l.Item(i) else { continue };
                        let Ok(vol) = d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { continue };
                        let id = d.GetId();
                        if let Ok(id) = id && let Ok(sid) = id.to_string() && let Ok(v) = vol.GetMute() {
                            sv.insert(sid, (id.0 as usize, v == TRUE));
                        } else if let Ok(id) = id { CoTaskMemFree(Some(id.0 as *const _)); }
                        let _ = vol.SetMute(true, null());
                    }
                } else {
                    let owned = std::mem::take(&mut sv);
                    for (_, (id, v)) in owned {
                        let id = PWSTR(id as *mut _);
                        let d = e.GetDevice(id);
                        CoTaskMemFree(Some(id.0 as *const _));
                        let Ok(d) = d else { continue };
                        let Ok(vol) = d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { continue };
                        let _ = vol.SetMute(v, null());
                    }
                    sv.clear();
                }
            }
        });

        let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kb_proc), Some(inst), 0).unwrap();
        let ms_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(ms_proc), Some(inst), 0).unwrap();
        let we_hook = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_REORDER, None, Some(we_proc), 0, 0, WINEVENT_OUTOFCONTEXT);
        let pwr = RegisterPowerSettingNotification(HANDLE(ol.0),
            &GUID_CONSOLE_DISPLAY_STATE,
            DEVICE_NOTIFY_WINDOW_HANDLE
        ).unwrap();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }
        UnhookWinEvent(we_hook).unwrap();
        UnhookWindowsHookEx(kb_hook).unwrap();
        UnhookWindowsHookEx(ms_hook).unwrap();
        UnregisterPowerSettingNotification(pwr).unwrap();
    });
    Ok(())
}

fn bitmap(s: usize) -> &'static mut [u32] {
    Box::leak(vec![0u32; (s + 31) / 32].into_boxed_slice())
}

enum EqMode { HAS, EQ }
fn bitmap_eq(m1: &[u32], m2: &[u32], m: EqMode) -> bool {
    match m {
        EqMode::HAS => { m1.iter().zip(m2.iter()).all(|(a, b)| a & b == *a) }
        EqMode::EQ => { m1 == m2 }
    }
}

fn bitmap_set(m: &mut [u32], i: usize, v: bool) {
    let idx = i >> 5;
    let pos = i & 31;
    if v {
        m[idx] |= 1 << pos;
    } else {
        m[idx] &= !(1 << pos);
    }
}

unsafe fn action(act: bool, ol_show: SHOW_WINDOW_CMD, tray: SHOW_WINDOW_CMD, winntf: isize) {
    let state = global_state();
    state.act.store(act, Ordering::Relaxed);
    let _ = ShowWindow(state.overlay, ol_show);
    find_show_win(w!("Shell_SecondaryTrayWnd"), tray);
    find_show_win(w!("Shell_TrayWnd"), tray);
    let _ = SendNotifyMessageW(HWND_BROADCAST, WM_SYSCOMMAND, WPARAM(SC_MONITORPOWER as usize), LPARAM(winntf));
}

fn bm_setup(m: &mut [u32], d: Vec<u16>) {
    for k in d { bitmap_set(m, k as usize, true); }
}

pub fn run(on: Vec<u16>, off: Vec<u16>) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<(usize, i32)>();
    let cond = Arc::new((Mutex::new(false), Condvar::new()));
    unsafe { init(tx, cond.clone())?; }
    let bmap_disable = bitmap(256); bm_setup(bmap_disable, off);
    let bmap_enable = bitmap(256); bm_setup(bmap_enable, on);
    let bmap = bitmap(256);
    let mut act = false;
    println!("Started");
    loop {
        if let Ok((key, value)) = rx.recv() {
            let value = value > 0;
            bitmap_set(bmap, key, value);
            if bitmap_eq(bmap_disable, bmap, if act { EqMode::EQ } else { EqMode::HAS }) {
                println!("Changed");
                act = !act;
                *cond.0.lock().unwrap() = act;
                cond.1.notify_all();
                unsafe{ if act {
                    action(true, SW_SHOWMAXIMIZED, SW_HIDE, 2);
                    ol_fix();
                    SetCursor(None);
                    let _ = EnumDisplayMonitors(None, None, Some(mon_proc), LPARAM(0));
                } else {
                    action(false, SW_HIDE, SW_SHOW, -1);
                }}
            }
        }
    }
}

pub fn clear() {
    eprintln!("Already clean");
}
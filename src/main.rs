use std::collections::HashMap;
use clap::{Parser, Subcommand};

#[cfg(target_os = "linux")] mod clk_linux;
#[cfg(target_os = "linux")] use clk_linux::{run, clear};
#[cfg(windows)] mod clk_windows;
#[cfg(windows)] use clk_windows::{run, clear};

fn keymap() -> HashMap<String, u16> {
    let mut m = HashMap::new();
    let mut add = |win: u16, linux: u16, mac: u16, names: &[&str]|  {
        let k = if cfg!(target_os = "windows") { win
        } else if cfg!(target_os = "macos") { mac
        } else { linux };
        for name in names.iter() { m.insert(name.to_string(), k); };
    };
    add(0x1B, 1,   53,  &["escape", "esc"]);
    add(0x0D, 28,  36,  &["enter"]);
    add(0x09, 15,  48,  &["tab"]);
    add(0x08, 14,  51,  &["backspace"]);
    add(0x20, 57,  49,  &["space"]);
    add(0x2D, 110, 114, &["insert", "ins"]);
    add(0x2E, 111, 117, &["delete", "del"]);
    add(0x24, 102, 115, &["home", "hm", "hom"]);
    add(0x23, 107, 119, &["end"]);
    add(0x21, 104, 116, &["pageup", "pgup", "pgu"]);
    add(0x22, 109, 121, &["pagedown", "pgdw", "pgd", "pgdwnn"]);
    add(0x25, 105, 123, &["arrowleft", "leftarrow", "larrow"]);
    add(0x26, 103, 126, &["arrowup", "uparrow", "uarrow"]);
    add(0x27, 106, 124, &["arrowright", "rightarrow", "rarrow"]);
    add(0x28, 108, 125, &["arrowdown", "downarrow", "darrow"]);
    add(0x41, 30,  0,   &["a"]);
    add(0x42, 48,  11,  &["b"]);
    add(0x43, 46,  8,   &["c"]);
    add(0x44, 32,  2,   &["d"]);
    add(0x45, 18,  14,  &["e"]);
    add(0x46, 33,  3,   &["f"]);
    add(0x47, 34,  5,   &["g"]);
    add(0x48, 35,  4,   &["h"]);
    add(0x49, 23,  34,  &["i"]);
    add(0x4A, 36,  38,  &["j"]);
    add(0x4B, 37,  40,  &["k"]);
    add(0x4C, 38,  37,  &["l"]);
    add(0x4D, 50,  46,  &["m"]);
    add(0x4E, 49,  45,  &["n"]);
    add(0x4F, 24,  31,  &["o"]);
    add(0x50, 25,  35,  &["p"]);
    add(0x51, 16,  12,  &["q"]);
    add(0x52, 19,  15,  &["r"]);
    add(0x53, 31,  1,   &["s"]);
    add(0x54, 20,  17,  &["t"]);
    add(0x55, 22,  32,  &["u"]);
    add(0x56, 47,  9,   &["v"]);
    add(0x57, 17,  13,  &["w"]);
    add(0x58, 45,  7,   &["x"]);
    add(0x59, 21,  16,  &["y"]);
    add(0x5A, 44,  6,   &["z"]);
    add(0x30, 11,  29,  &["d0", "0"]);
    add(0x31, 2,   18,  &["d1", "1"]);
    add(0x32, 3,   19,  &["d2", "2"]);
    add(0x33, 4,   20,  &["d3", "3"]);
    add(0x34, 5,   21,  &["d4", "4"]);
    add(0x35, 6,   23,  &["d5", "5"]);
    add(0x36, 7,   22,  &["d6", "6"]);
    add(0x37, 8,   26,  &["d7", "7"]);
    add(0x38, 9,   28,  &["d8", "8"]);
    add(0x39, 10,  25,  &["d9", "9"]);
    add(0x60, 82,  82,  &["num0", "n0"]);
    add(0x61, 79,  83,  &["num1", "n1"]);
    add(0x62, 80,  84,  &["num2", "n2"]);
    add(0x63, 81,  85,  &["num3", "n3"]);
    add(0x64, 75,  86,  &["num4", "n4"]);
    add(0x65, 76,  87,  &["num5", "n5"]);
    add(0x66, 77,  88,  &["num6", "n6"]);
    add(0x67, 71,  89,  &["num7", "n7"]);
    add(0x68, 72,  91,  &["num8", "n8"]);
    add(0x69, 73,  92,  &["num9", "n9"]);
    add(0x6A, 55,  67,  &["nummultiply", "nmultiply", "nummul", "nmul", "num*", "n*"]);
    add(0x6B, 78,  69,  &["numadd", "nadd", "num+", "n+"]);
    add(0x6D, 74,  78,  &["numsubtract", "nsubtract", "numsub", "nsub", "num-", "n-"]);
    add(0x6F, 53,  75,  &["numdivide", "ndivide", "numdiv", "ndiv", "num/", "n/"]);
    add(0x6E, 83,  65,  &["numdecimal", "ndecimal", "numdec", "ndec", "numdelete", "ndelete", "numdel", "ndel", "numdot", "ndot", "num.", "n."]);
    add(0x70, 59,  122, &["f1"]);
    add(0x71, 60,  120, &["f2"]);
    add(0x72, 61,  99,  &["f3"]);
    add(0x73, 62,  118, &["f4"]);
    add(0x74, 63,  96,  &["f5"]);
    add(0x75, 64,  97,  &["f6"]);
    add(0x76, 65,  98,  &["f7"]);
    add(0x77, 66,  100, &["f8"]);
    add(0x78, 67,  101, &["f9"]);
    add(0x79, 68,  109, &["f10"]);
    add(0x7A, 87,  103, &["f11"]);
    add(0x7B, 88,  111, &["f12"]);
    add(0x7C, 183, 104, &["f13"]);
    add(0x7D, 184, 105, &["f14"]);
    add(0x7E, 185, 106, &["f15"]);
    add(0x7F, 186, 107, &["f16"]);
    add(0x80, 187, 108, &["f17"]);
    add(0x81, 188, 110, &["f18"]);
    add(0x82, 189, 112, &["f19"]);
    add(0x83, 190, 113, &["f20"]);
    add(0x84, 191, 102, &["f21"]);
    add(0x85, 192, 121, &["f22"]);
    add(0x86, 193, 122, &["f23"]);
    add(0x87, 194, 123, &["f24"]);
    add(0x14, 58,  57,  &["capslock", "caps", "cl"]);
    add(0x90, 69,  71,  &["numlock", "num", "nl"]);
    add(0x91, 70,  72,  &["scrolllock", "scroll", "scrl", "sl"]);
    add(0x13, 119, 117, &["pausebreak", "pause", "pb", "pauseb", "pbreak", "break"]);
    add(0x5D, 135, 110, &["apps"]);
    add(0x5F, 142, 142, &["sleep"]);
    add(0xB3, 164, 163, &["mediaplaystop"]);
    add(0xB2, 165, 165, &["mediastop"]);
    add(0xB0, 163, 171, &["medianext"]);
    add(0xB1, 166, 172, &["mediaprevious", "mediaprev"]);
    add(0xAF, 123, 126, &["volumeup", "volup", "vup"]);
    add(0xAE, 122, 125, &["volumedown", "voldown", "vdown", "volumed", "vold", "vd", "volumedw", "voldw", "vdw"]);
    add(0xAD, 121, 127, &["volumemute", "volmute", "vmute"]);
    add(0xA6, 166, 178, &["browserback"]);
    add(0xA7, 167, 179, &["browserforward"]);
    add(0xA8, 168, 177, &["browserrefresh"]);
    add(0xAC, 178, 174, &["browserhome"]);
    add(0xA4, 56,  61,  &["leftalt", "lalt", "alt"]);
    add(0xA5, 100, 61,  &["rightalt", "ralt"]);
    add(0xA2, 29,  59,  &["leftcontrol", "lcontrol", "control", "leftctrl", "lctrl", "ctrl"]);
    add(0xA3, 97,  59,  &["rightcontrol", "rcontrol", "rightctrl", "rctrl"]);
    add(0xA0, 42,  60,  &["leftshift", "lshift", "shift"]);
    add(0xA1, 54,  60,  &["rightshift", "rshift"]);
    add(0x5B, 125, 55,  &["leftmeta", "lmeta", "meta", "leftwin", "lwin", "win"]);
    add(0x5C, 126, 55,  &["rightmeta", "rmeta", "rightwin", "rwin"]);
    add(0x2C, 107, 105, &["printscreen", "prtsc", "prscrn", "prtscr", "prsc"]);
    m
}

fn kcodes(keymap: &HashMap<String, u16>, s: &str) -> Vec<u16> {
    s.split(',').map(|k| {
        let err = format!("Invalid key: {}", k);
        let n = k.to_lowercase();
        *keymap.get(n.trim()).expect(&err)
    }).collect::<Vec<_>>()
}

#[derive(Parser, Debug)]
#[command(author, version, about = "ClkMonOff by MeowKotuk606", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Key names delimited by ","
    #[arg(long, required_unless_present = "command")]
    start: Option<String>,

    /// Key names delimited by ","
    #[arg(long, required_unless_present = "command")]
    end: Option<String>,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// Purge all application data and cache
    Clear,
}

fn main() {
    let args = Args::parse();
    if Some(Commands::Clear) == args.command {
        clear();
    } else if let Some(start) = &args.start && let Some(end) = &args.end {
        let (start, end) = {
            let km = keymap();
            (kcodes(&km, start), kcodes(&km, end))
        };
        drop(args);
        run(start, end).unwrap();
    } else {
        eprintln!("Invalid command, use --help");
    }
}

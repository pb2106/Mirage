use rand::seq::SliceRandom;
use rand::Rng;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub has_macos_share: bool,
    pub primary_lang: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NicCategory {
    LaptopWifi,
    DesktopEthernet,
    VmVirtio,
    PhoneWifi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacAddress(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Laptop,
    Desktop,
    Phone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleCombo {
    pub lang: String,
    pub lc_time: String,
    pub lc_numeric: String,
    pub tz: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitPersona {
    pub user_name: String,
    pub user_email: String,
}

pub fn generate_hostname(region: &Region, rng: &mut impl Rng) -> String {
    let patterns = if region.has_macos_share {
        vec!["PatternA", "PatternB", "PatternC", "PatternD"]
    } else {
        vec!["PatternA", "PatternC", "PatternD"]
    };

    let choice = patterns.choose(rng).unwrap();
    match *choice {
        "PatternA" => {
            let names = ["priya", "james", "alex", "sakura", "chen"];
            let models = ["thinkpad", "latitude", "xps", "elitebook"];
            format!("{}-{}", names.choose(rng).unwrap(), models.choose(rng).unwrap())
        }
        "PatternB" => {
            let names = ["priya", "james", "alex", "sakura", "chen"];
            format!("{}s-macbook-pro", names.choose(rng).unwrap())
        }
        "PatternD" => {
            let depts = ["fin", "eng", "hr", "sales"];
            format!("{}-ws-{:04}", depts.choose(rng).unwrap(), rng.gen_range(1..9999))
        }
        _ => { // Pattern C
            let words = ["archlinux", "workstation", "dev-box", "ninja"];
            format!("{}-{}", words.choose(rng).unwrap(), rng.gen_range(1..100))
        }
    }
}

pub fn generate_machine_id(rng: &mut impl Rng) -> MachineId {
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    // Set UUIDv4 version and variant bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let u = uuid::Uuid::from_bytes(bytes);
    MachineId(u.to_string().replace("-", ""))
}

pub fn generate_mac(category: NicCategory, _region: &Region, rng: &mut impl Rng) -> MacAddress {
    let oui = match category {
        NicCategory::LaptopWifi => vec![[0x00, 0x11, 0x24], [0x00, 0x14, 0x22]],
        NicCategory::DesktopEthernet => vec![[0x00, 0x1C, 0xC0], [0x00, 0x1B, 0x21]],
        NicCategory::VmVirtio => vec![[0x52, 0x54, 0x00], [0x08, 0x00, 0x27], [0x00, 0x05, 0x69]],
        NicCategory::PhoneWifi => vec![[0x00, 0x22, 0xAB], [0x00, 0x11, 0x22]],
    };
    
    let chosen_oui = oui.choose(rng).unwrap();
    let b1 = chosen_oui[0] & 0xFD; // Ensure locally administered bit is 0
    let b2 = chosen_oui[1];
    let b3 = chosen_oui[2];
    
    MacAddress(format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        b1, b2, b3, rng.gen::<u8>(), rng.gen::<u8>(), rng.gen::<u8>()
    ))
}

pub fn generate_screen_resolution(device_class: DeviceClass, _region: &Region, rng: &mut impl Rng) -> Resolution {
    match device_class {
        DeviceClass::Laptop => {
            let res = vec![(2560, 1600), (1920, 1200), (1920, 1080), (1366, 768), (2880, 1800)];
            let chosen = res.choose(rng).unwrap();
            Resolution { width: chosen.0, height: chosen.1 }
        }
        DeviceClass::Desktop => {
            let res = vec![(3840, 2160), (2560, 1440), (1920, 1080), (1680, 1050)];
            let chosen = res.choose(rng).unwrap();
            Resolution { width: chosen.0, height: chosen.1 }
        }
        DeviceClass::Phone => {
            let res = vec![(1080, 2340), (1170, 2532), (1440, 3200)];
            let chosen = res.choose(rng).unwrap();
            Resolution { width: chosen.0, height: chosen.1 }
        }
    }
}

pub fn generate_font_set(_region: &Region, rng: &mut impl Rng) -> Vec<FontName> {
    let mut fonts = vec![
        "Arial", "Times New Roman", "Courier New", "Verdana", "Tahoma", "Trebuchet MS", "Georgia", "Impact", "Comic Sans MS", "Lucida Console", "Garamond", "Palatino Linotype"
    ];
    let supp = vec!["Noto Sans CJK JP", "Noto Sans Arabic", "Noto Sans Devanagari", "FreeSans", "DejaVu Sans"];
    let apps = vec!["Liberation Sans", "Ubuntu", "Droid Sans", "Roboto", "Open Sans"];
    
    for _ in 0..rng.gen_range(1..3) {
        fonts.push(supp.choose(rng).unwrap());
    }
    for _ in 0..rng.gen_range(3..6) {
        fonts.push(apps.choose(rng).unwrap());
    }
    
    fonts.shuffle(rng);
    fonts.into_iter().map(|s| FontName(s.to_string())).collect()
}

pub fn generate_locale_combo(region: &Region, rng: &mut impl Rng) -> LocaleCombo {
    let mut lang = format!("{}.UTF-8", region.primary_lang);
    let mut lc_time = lang.clone();
    let mut lc_numeric = lang.clone();
    
    if rng.gen_bool(0.2) {
        // variation
        if rng.gen_bool(0.5) {
            lc_time = "en_US.UTF-8".to_string();
        } else {
            lc_numeric = "de_DE.UTF-8".to_string();
        }
    }
    
    LocaleCombo {
        lang,
        lc_time,
        lc_numeric,
        tz: "Europe/London".to_string(),
    }
}

pub fn generate_git_persona(region: &Region, rng: &mut impl Rng) -> GitPersona {
    let names = ["Priya", "James", "Alex", "Sakura", "Chen", "Muhammad", "Wei"];
    let domains = ["gmail.com", "yahoo.co.jp", "naver.com", "163.com", "gmx.de"];
    
    let first = names.choose(rng).unwrap();
    let last = names.choose(rng).unwrap(); // just pick a random last name for simplicity
    
    GitPersona {
        user_name: format!("{} {}", first, last),
        user_email: format!("{}.{}@{}", first.to_lowercase(), last.to_lowercase(), domains.choose(rng).unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    
    #[test]
    fn test_generate_hostname() {
        let mut rng = StdRng::seed_from_u64(42);
        let region = Region { name: "Test".into(), has_macos_share: true, primary_lang: "en_US".into() };
        let mut hostnames = std::collections::HashSet::new();
        for _ in 0..1000 {
            hostnames.insert(generate_hostname(&region, &mut rng));
        }
        assert!(hostnames.len() > 4); // Multiple patterns generate multiple things
    }

    #[test]
    fn test_generate_mac() {
        let mut rng = StdRng::seed_from_u64(42);
        let region = Region { name: "Test".into(), has_macos_share: true, primary_lang: "en_US".into() };
        let mac = generate_mac(NicCategory::LaptopWifi, &region, &mut rng);
        assert!(!mac.0.starts_with("52:54:00")); // not VM
        let b1 = u8::from_str_radix(&mac.0[0..2], 16).unwrap();
        assert_eq!(b1 & 0x02, 0); // locally administered bit is 0
    }
}

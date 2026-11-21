//! Settings → Language and Time and place → Formats: spelling dictionaries,
//! the regional formats (LC_TIME & co.) and the apps' language packs.
//!
//! The spelling languages are a pref in ewe.conf (`[desktop.language]
//! spellcheck`, written through ewe-conf by the pane — it syncs, so a name
//! with no dictionary on this machine is shown as "not on this computer",
//! never an error). Dictionaries, formats and language packs are machine
//! state: hunspell files in /usr/share/hunspell, localectl, pacman. Installs
//! go through `backend::install_packages` (one polkit prompt); a dictionary
//! that only the AUR has opens Komble's search instead.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Stdio;

use serde_json::{json, Value};
use tokio::process::Command;

use crate::backend::{
    install_name_ok, locale_gen_part, locale_label, locale_value_ok, pkexec_script, run_out,
    set_locale_script, supported_locales, system_locale_vars, valid_locale,
};
use crate::util::estr;

const DICT_DIRS: &[&str] = &[
    "/usr/share/hunspell",
    "/usr/share/myspell/dicts",
    "/usr/share/myspell",
];

/// Every dictionary the official repositories install, with its package
/// (from the packages' file lists, 2026-11-21) and whether it is offered on
/// its own in "Add a language" — hunspell-en_gb alone brings 19 English
/// dictionaries; the sheet lists the main one of each package.
const OFFICIAL: &[(&str, &str, bool)] = &[
    ("de_DE", "hunspell-de", true),
    ("de_AT", "hunspell-de", true),
    ("de_CH", "hunspell-de", true),
    ("de_BE", "hunspell-de", false),
    ("de_LI", "hunspell-de", false),
    ("de_LU", "hunspell-de", false),
    ("el_GR", "hunspell-el", true),
    ("en_US", "hunspell-en_us", true),
    ("en_GB", "hunspell-en_gb", true),
    ("en_AU", "hunspell-en_au", true),
    ("en_CA", "hunspell-en_ca", true),
    ("en_IE", "hunspell-en_gb", false),
    ("en_NZ", "hunspell-en_gb", false),
    ("en_ZA", "hunspell-en_gb", false),
    ("en_IN", "hunspell-en_gb", false),
    ("en_SG", "hunspell-en_gb", false),
    ("en_HK", "hunspell-en_gb", false),
    ("en_DK", "hunspell-en_gb", false),
    ("en_NG", "hunspell-en_gb", false),
    ("en_GH", "hunspell-en_gb", false),
    ("en_JM", "hunspell-en_gb", false),
    ("en_TT", "hunspell-en_gb", false),
    ("en_ZW", "hunspell-en_gb", false),
    ("en_BW", "hunspell-en_gb", false),
    ("en_NA", "hunspell-en_gb", false),
    ("en_BZ", "hunspell-en_gb", false),
    ("en_BS", "hunspell-en_gb", false),
    ("en_AG", "hunspell-en_gb", false),
    ("es_ES", "hunspell-es_es", true),
    ("es_MX", "hunspell-es_mx", true),
    ("es_AR", "hunspell-es_ar", true),
    ("es_BO", "hunspell-es_bo", true),
    ("es_CL", "hunspell-es_cl", true),
    ("es_CO", "hunspell-es_co", true),
    ("es_CR", "hunspell-es_cr", true),
    ("es_CU", "hunspell-es_cu", true),
    ("es_DO", "hunspell-es_do", true),
    ("es_EC", "hunspell-es_ec", true),
    ("es_GT", "hunspell-es_gt", true),
    ("es_HN", "hunspell-es_hn", true),
    ("es_NI", "hunspell-es_ni", true),
    ("es_PA", "hunspell-es_pa", true),
    ("es_PE", "hunspell-es_pe", true),
    ("es_PR", "hunspell-es_pr", true),
    ("es_PY", "hunspell-es_py", true),
    ("es_SV", "hunspell-es_sv", true),
    ("es_UY", "hunspell-es_uy", true),
    ("es_VE", "hunspell-es_ve", true),
    ("es", "hunspell-es_any", false),
    ("fr_FR", "hunspell-fr-comprehensive", true),
    ("fr_BE", "hunspell-fr-comprehensive", false),
    ("fr_CA", "hunspell-fr-comprehensive", true),
    ("fr_CH", "hunspell-fr-comprehensive", false),
    ("fr_LU", "hunspell-fr-comprehensive", false),
    ("he_IL", "hunspell-he", true),
    ("hu_HU", "hunspell-hu", true),
    ("it_IT", "hunspell-it", true),
    ("it_CH", "hunspell-it", false),
    ("nl_NL", "hunspell-nl", true),
    ("nl_BE", "hunspell-nl", false),
    ("nl_AW", "hunspell-nl", false),
    ("pl_PL", "hunspell-pl", true),
    ("ro_RO", "hunspell-ro", true),
    ("ru_RU", "hunspell-ru", true),
];

/// Languages whose dictionary only the AUR has (Komble builds it): the
/// language code, its usual dictionary name, the AUR package.
const AUR: &[(&str, &str, &str)] = &[
    ("af", "af_ZA", "hunspell-af"),
    ("ar", "ar", "hunspell-ar"),
    ("be", "be_BY", "hunspell-be"),
    ("bg", "bg_BG", "hunspell-bg"),
    ("bn", "bn_BD", "hunspell-bn"),
    ("ca", "ca_ES", "hunspell-ca"),
    ("cs", "cs_CZ", "hunspell-cs"),
    ("eo", "eo", "hunspell-eo"),
    ("et", "et_EE", "hunspell-et"),
    ("fa", "fa_IR", "hunspell-fa"),
    ("fo", "fo_FO", "hunspell-fo"),
    ("ga", "ga_IE", "hunspell-ga"),
    ("gl", "gl_ES", "hunspell-gl"),
    ("hi", "hi_IN", "hunspell-hi"),
    ("hr", "hr_HR", "hunspell-hr"),
    ("hy", "hy_AM", "hunspell-hy-reformed"),
    ("id", "id_ID", "hunspell-id"),
    ("is", "is_IS", "hunspell-is"),
    ("ka", "ka_GE", "hunspell-ka"),
    ("ko", "ko_KR", "hunspell-ko"),
    ("lb", "lb_LU", "hunspell-lb"),
    ("lt", "lt_LT", "hunspell-lt"),
    ("lv", "lv_LV", "hunspell-lv"),
    ("nb", "nb_NO", "hunspell-nb"),
    ("nn", "nn_NO", "hunspell-nn"),
    ("pt_BR", "pt_BR", "hunspell-pt-br"),
    ("pt_PT", "pt_PT", "hunspell-pt_pt"),
    ("pt", "pt_PT", "hunspell-pt_pt"),
    ("si", "si_LK", "hunspell-si"),
    ("sk", "sk_SK", "hunspell-sk"),
    ("sl", "sl_SI", "hunspell-sl"),
    ("sr", "sr", "hunspell-sr"),
    ("sv", "sv_SE", "hunspell-sv"),
    ("th", "th_TH", "hunspell-th"),
    ("tr", "tr_TR", "hunspell-tr"),
    ("uk", "uk_UA", "hunspell-uk"),
    ("ur", "ur_PK", "hunspell-ur"),
    ("vi", "vi_VN", "hunspell-vi"),
];

/// The shape ewe-conf accepts for `desktop.language.spellcheck` (its
/// `_DICT_NAME`): en_US, es, sr-Latn, ro_RO-ante1993.
pub(crate) fn dict_name_ok(d: &str) -> bool {
    let (base, variant) = match d.split_once('-') {
        Some((b, v)) => (b, Some(v)),
        None => (d, None),
    };
    let (lang, region) = match base.split_once('_') {
        Some((l, r)) => (l, Some(r)),
        None => (base, None),
    };
    (2..=3).contains(&lang.len())
        && lang.chars().all(|c| c.is_ascii_lowercase())
        && region.is_none_or(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()))
        && variant.is_none_or(|v| {
            (2..=12).contains(&v.len()) && v.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

/// "de_DE.UTF-8@euro" → "de_DE"
fn locale_base(locale: &str) -> &str {
    let l = locale.split('@').next().unwrap_or(locale);
    l.split('.').next().unwrap_or(l)
}

/// Dictionaries installed here: a .dic with its .aff, the plain ones only
/// (no "-large"/"-ante1993" variants), each name once, sorted.
pub(crate) fn installed_dicts_in(dirs: &[PathBuf]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(stem) = name.strip_suffix(".dic") else {
                continue;
            };
            if stem.contains('-') || !dict_name_ok(stem) {
                continue;
            }
            if dir.join(format!("{stem}.aff")).exists() {
                out.insert(stem.to_string());
            }
        }
    }
    out.into_iter().collect()
}

fn installed_dicts() -> Vec<String> {
    installed_dicts_in(&DICT_DIRS.iter().map(PathBuf::from).collect::<Vec<_>>())
}

/// Where a dictionary comes from: ("repo", package) for the official ones,
/// ("aur", package) for a language only the AUR has, None otherwise.
pub(crate) fn dict_source(d: &str) -> Option<(&'static str, &'static str)> {
    if let Some((_, p, _)) = OFFICIAL.iter().find(|(c, _, _)| *c == d) {
        return Some(("repo", p));
    }
    let base = d.split('-').next().unwrap_or(d);
    let lang = base.split('_').next().unwrap_or(base);
    AUR.iter()
        .find(|(l, _, _)| *l == base)
        .or_else(|| AUR.iter().find(|(l, _, _)| *l == lang))
        .map(|(_, _, p)| ("aur", *p))
}

/// The dictionary for a locale: its own name when one exists (installed,
/// official or AUR), else the first of the same language — "de_DE.UTF-8" →
/// de_DE, "en_PH.UTF-8" → en_US, "ka_GE.UTF-8" → ka_GE.
pub(crate) fn dict_for_locale(locale: &str, installed: &[String]) -> Option<String> {
    let base = locale_base(locale);
    let lang = base.split('_').next().unwrap_or(base);
    if lang.is_empty() || lang == "C" || lang == "POSIX" {
        return None;
    }
    if installed.iter().any(|d| d == base) || OFFICIAL.iter().any(|(c, _, _)| *c == base) {
        return Some(base.to_string());
    }
    if let Some((_, d, _)) = AUR.iter().find(|(l, _, _)| *l == base || *l == lang) {
        return Some((*d).to_string());
    }
    // the same language, another region: installed first, then official —
    // en_US before the rest of English
    let same = |d: &str| d.split('_').next() == Some(lang);
    if let Some(d) = installed.iter().find(|d| same(d)) {
        return Some(d.clone());
    }
    let pref = format!("{lang}_{}", lang.to_uppercase());
    if OFFICIAL.iter().any(|(c, _, _)| *c == pref) {
        return Some(pref);
    }
    if lang == "en" {
        return Some("en_US".into());
    }
    OFFICIAL
        .iter()
        .find(|(c, _, offered)| *offered && same(c))
        .map(|(c, _, _)| (*c).to_string())
}

/// No list in ewe.conf: the system language's dictionary (when installed
/// here) and US English.
pub(crate) fn automatic_dicts(lang: &str, installed: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(d) = dict_for_locale(lang, installed) {
        if installed.contains(&d) {
            out.push(d);
        }
    }
    if !out.iter().any(|d| d == "en_US") {
        out.push("en_US".into());
    }
    out
}

fn dict_label(d: &str) -> String {
    match d {
        "es" => "Spanish (all countries)".into(),
        "ar" => "Arabic".into(),
        "eo" => "Esperanto".into(),
        "sr" => "Serbian".into(),
        _ => locale_label(d),
    }
}

async fn conf_get(key: &str) -> Option<String> {
    let bin = crate::backend::ewe_tool("ewe-conf")?;
    let out = Command::new(bin)
        .args(["get", key])
        .stdin(Stdio::null())
        .output()
        .await
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn current_lang(vars: &[(String, String)]) -> String {
    vars.iter()
        .find(|(k, _)| k == "LANG")
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// Everything the Spell checking group shows.
#[tauri::command]
pub async fn spell_info() -> Result<Value, String> {
    let installed = installed_dicts();
    let lang = current_lang(&system_locale_vars().await);
    let enabled = conf_get("desktop.language.check_spelling")
        .await
        .is_none_or(|v| v != "false");
    let saved: Option<Vec<String>> = conf_get("desktop.language.spellcheck")
        .await
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .map(|l| l.into_iter().filter(|d| dict_name_ok(d)).collect());
    let auto = saved.is_none();
    let chosen = saved.unwrap_or_else(|| automatic_dicts(&lang, &installed));
    let row = |d: &str, state: &str| {
        let (source, package) =
            dict_source(d).map_or((Value::Null, Value::Null), |(s, p)| (json!(s), json!(p)));
        json!({ "code": d, "label": dict_label(d), "state": state, "source": source, "package": package })
    };
    let chosen_rows: Vec<Value> = chosen
        .iter()
        .map(|d| {
            row(
                d,
                if installed.contains(d) {
                    "installed"
                } else {
                    "missing"
                },
            )
        })
        .collect();
    // "Add a language": installed first, then the official downloads, then
    // the AUR's (Komble); the system language's dictionary leads its kind
    let system = dict_for_locale(&lang, &installed);
    let mut available: Vec<Value> = Vec::new();
    for d in installed.iter().filter(|d| !chosen.contains(d)) {
        available.push(row(d, "installed"));
    }
    let pkg_installed: BTreeSet<&str> = installed
        .iter()
        .filter_map(|d| OFFICIAL.iter().find(|(c, _, _)| c == d).map(|(_, p, _)| *p))
        .collect();
    for (d, p, offered) in OFFICIAL {
        let is_system = system.as_deref() == Some(*d);
        if (!offered && !is_system) || installed.iter().any(|i| i == d) || pkg_installed.contains(p)
        {
            continue;
        }
        if chosen.iter().any(|c| c == d) {
            continue;
        }
        available.push(row(d, "repo"));
    }
    let mut aur_seen = BTreeSet::new();
    for (_, d, p) in AUR {
        if !aur_seen.insert(*p) || installed.iter().any(|i| i == d) || chosen.iter().any(|c| c == d)
        {
            continue;
        }
        available.push(row(d, "aur"));
    }
    if let Some(sys) = system.as_deref() {
        if let Some(i) = available.iter().position(|v| v["code"] == sys) {
            let v = available.remove(i);
            available.insert(0, v);
        }
    }
    Ok(json!({
        "enabled": enabled,
        "auto": auto,
        "chosen": chosen_rows,
        "available": available,
        "system": system,
        "systemLabel": system.as_deref().map(dict_label),
        "confAvailable": crate::backend::ewe_tool("ewe-conf").is_some(),
    }))
}

/// Open Komble's search on a package (a dictionary only the AUR has).
/// `Ok(false)` when Komble is not installed.
#[tauri::command]
pub async fn komble_search(query: String) -> Result<bool, String> {
    if !install_name_ok(&query) {
        return Err("bad search".into());
    }
    let bin = PathBuf::from("/usr/bin/komble");
    if !bin.exists() {
        return Ok(false);
    }
    Command::new(bin)
        .arg(format!("--search={query}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(estr)?;
    Ok(true)
}

// ── Regional formats ─────────────────────────────────────────────────────────
// Separate from the display language (LANG): dates and times, numbers, money,
// measurement and paper (the five below) come from one region. GTK and glibc
// read them at start; Qt reads them through QLocale::system(), so the shell's
// clock and calendar follow at the next sign-in.

pub(crate) const FORMAT_VARS: &[&str] = &[
    "LC_TIME",
    "LC_NUMERIC",
    "LC_MONETARY",
    "LC_MEASUREMENT",
    "LC_PAPER",
];

/// The formats locale: LC_TIME, or "" when the formats follow the language.
/// `mixed` when the five do not agree (set by hand) — the pane says so.
pub(crate) fn formats_of(vars: &[(String, String)]) -> (String, bool) {
    let get = |k: &str| vars.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let time = get("LC_TIME").unwrap_or_default();
    let mixed = FORMAT_VARS
        .iter()
        .any(|k| get(k).unwrap_or_default() != time);
    (time, mixed)
}

/// The full variable set after a formats change: LANG and everything that is
/// not a format kept, then the five (left out for "" = like the language).
pub(crate) fn with_formats(vars: &[(String, String)], formats: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vars
        .iter()
        .filter(|(k, v)| !FORMAT_VARS.contains(&k.as_str()) && locale_value_ok(k, v))
        .cloned()
        .collect();
    if !formats.is_empty() {
        out.extend(
            FORMAT_VARS
                .iter()
                .map(|k| (k.to_string(), formats.to_string())),
        );
    }
    out
}

#[tauri::command]
pub async fn formats_info() -> Result<Value, String> {
    let vars = system_locale_vars().await;
    let lang = current_lang(&vars);
    let (formats, mixed) = formats_of(&vars);
    Ok(json!({
        "lang": lang,
        "langLabel": locale_label(&lang),
        "formats": formats,
        "formatsLabel": if formats.is_empty() { String::new() } else { locale_label(&formats) },
        "mixed": mixed,
    }))
}

/// One pkexec prompt: generate the locale if needed, then `localectl
/// set-locale` with LANG kept and the five formats set ("" clears them, so
/// the formats follow the language again).
#[tauri::command]
pub async fn set_formats(locale: String) -> Result<(), String> {
    let l = locale.trim().to_string();
    if !l.is_empty() && (!valid_locale(&l) || !supported_locales().contains(&l)) {
        return Err(format!("unknown region: {l}"));
    }
    let vars = with_formats(&system_locale_vars().await, &l);
    if vars.is_empty() {
        return Err("no language is set yet — pick one first".into());
    }
    let gen = if l.is_empty() {
        String::new()
    } else {
        locale_gen_part(&l).await
    };
    pkexec_script(&set_locale_script(&gen, &vars), "set the formats").await
}

// ── The apps' language packs ─────────────────────────────────────────────────
// Firefox, Thunderbird and LibreOffice speak a language only with its pack
// installed. Offered for the apps that ARE installed, matched to a locale:
// "<prefix><ll>-<cc>", "<prefix><ll>", "<prefix><ll>-<ll>", then the first of
// the language ("es_CO" → firefox-i18n-es-es). US English is every app's own.

const PACK_APPS: &[(&str, &str, &str)] = &[
    ("firefox", "Firefox", "firefox-i18n-"),
    ("thunderbird", "Thunderbird", "thunderbird-i18n-"),
    ("libreoffice-fresh", "LibreOffice", "libreoffice-fresh-"),
    ("libreoffice-still", "LibreOffice", "libreoffice-still-"),
];

pub(crate) fn pick_pack(
    prefix: &str,
    locale: &str,
    available: &BTreeSet<String>,
) -> Option<String> {
    let base = locale_base(locale).to_lowercase();
    let (ll, cc) = match base.split_once('_') {
        Some((l, c)) => (l.to_string(), c.to_string()),
        None => (base.clone(), String::new()),
    };
    if ll.is_empty() || (ll == "en" && (cc.is_empty() || cc == "us")) {
        return None;
    }
    let mut cands = vec![];
    if !cc.is_empty() {
        cands.push(format!("{prefix}{ll}-{cc}"));
    }
    cands.push(format!("{prefix}{ll}"));
    cands.push(format!("{prefix}{ll}-{ll}"));
    if let Some(c) = cands.into_iter().find(|c| available.contains(c)) {
        return Some(c);
    }
    let lead = format!("{prefix}{ll}-");
    available
        .iter()
        .find(|a| a.starts_with(&lead) && !a[lead.len()..].contains('-'))
        .cloned()
}

/// `pacman -Q a b c` → the names that are installed
async fn installed_of(names: &[String]) -> BTreeSet<String> {
    if names.is_empty() {
        return BTreeSet::new();
    }
    let mut args = vec!["-Q", "--"];
    args.extend(names.iter().map(String::as_str));
    run_out("pacman", &args)
        .await
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with("error:"))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect()
}

#[tauri::command]
pub async fn language_packs(locale: String) -> Result<Value, String> {
    let l = locale.trim().to_string();
    if !valid_locale(&l) {
        return Err(format!("unknown locale: {l}"));
    }
    let apps: Vec<String> = PACK_APPS.iter().map(|(a, _, _)| a.to_string()).collect();
    let have_apps = installed_of(&apps).await;
    let mut packs: Vec<(String, String)> = Vec::new(); // (label, package)
    if PACK_APPS.iter().any(|(a, _, _)| have_apps.contains(*a)) {
        let avail: BTreeSet<String> = run_out(
            "pacman",
            &[
                "-Ssq",
                "^(firefox-i18n|thunderbird-i18n|libreoffice-fresh|libreoffice-still)-",
            ],
        )
        .await
        .unwrap_or_default()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
        for (app, label, prefix) in PACK_APPS {
            if !have_apps.contains(*app) {
                continue;
            }
            if let Some(p) = pick_pack(prefix, &l, &avail) {
                packs.push((label.to_string(), p));
            }
        }
    }
    // the language's spelling dictionary, when the official repos have it
    let installed = installed_dicts();
    if let Some(d) = dict_for_locale(&l, &installed) {
        if !installed.contains(&d) {
            if let Some(("repo", p)) = dict_source(&d) {
                packs.push(("Spell checking".into(), p.to_string()));
            }
        }
    }
    let names: Vec<String> = packs.iter().map(|(_, p)| p.clone()).collect();
    let have = installed_of(&names).await;
    let rows: Vec<Value> = packs
        .into_iter()
        .map(|(label, p)| json!({ "app": label, "package": p, "installed": have.contains(&p) }))
        .collect();
    Ok(json!({ "locale": l, "label": locale_label(&l), "packs": rows }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_names() {
        for ok in ["en_US", "es", "sr-Latn", "ro_RO-ante1993", "ka_GE", "fil"] {
            assert!(dict_name_ok(ok), "{ok}");
        }
        for bad in [
            "EN", "en_us", "en_US;rm", "../x", "", "en_USA", "e", "en_US-",
        ] {
            assert!(!dict_name_ok(bad), "{bad}");
        }
    }

    #[test]
    fn installed_dictionaries_need_their_aff_and_skip_variants() {
        let dir = std::env::temp_dir().join(format!("ewe-settings-dicts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "en_US.dic",
            "en_US.aff",
            "en_US-large.dic",
            "en_US-large.aff",
            "de_DE.dic",
            "de_DE.aff",
            "orphan_XX.dic",
            "hyph_en_US.dic",
        ] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let got = installed_dicts_in(&[dir.clone(), dir.join("nowhere")]);
        assert_eq!(got, vec!["de_DE".to_string(), "en_US".to_string()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_dictionary_for_a_locale() {
        let none: Vec<String> = vec![];
        assert_eq!(
            dict_for_locale("de_DE.UTF-8", &none).as_deref(),
            Some("de_DE")
        );
        assert_eq!(
            dict_for_locale("de_AT.UTF-8", &none).as_deref(),
            Some("de_AT")
        );
        assert_eq!(
            dict_for_locale("ka_GE.UTF-8", &none).as_deref(),
            Some("ka_GE")
        );
        assert_eq!(
            dict_for_locale("en_PH.UTF-8", &none).as_deref(),
            Some("en_US")
        );
        assert_eq!(
            dict_for_locale("ru_UA.UTF-8", &none).as_deref(),
            Some("ru_RU")
        );
        assert_eq!(
            dict_for_locale("pt_BR.UTF-8", &none).as_deref(),
            Some("pt_BR")
        );
        assert_eq!(dict_for_locale("C.UTF-8", &none), None);
        let inst = vec!["en_GB".to_string()];
        assert_eq!(
            dict_for_locale("en_PH.UTF-8", &inst).as_deref(),
            Some("en_GB")
        );
    }

    #[test]
    fn where_a_dictionary_comes_from() {
        assert_eq!(dict_source("de_CH"), Some(("repo", "hunspell-de")));
        assert_eq!(dict_source("en_IE"), Some(("repo", "hunspell-en_gb")));
        assert_eq!(dict_source("ka_GE"), Some(("aur", "hunspell-ka")));
        assert_eq!(dict_source("pt_BR"), Some(("aur", "hunspell-pt-br")));
        assert_eq!(dict_source("xx_YY"), None);
    }

    #[test]
    fn automatic_list_is_the_system_language_when_installed_plus_us_english() {
        let inst = vec!["de_DE".to_string(), "en_US".to_string()];
        assert_eq!(
            automatic_dicts("de_DE.UTF-8", &inst),
            vec!["de_DE", "en_US"]
        );
        assert_eq!(automatic_dicts("ka_GE.UTF-8", &inst), vec!["en_US"]);
        assert_eq!(automatic_dicts("en_US.UTF-8", &inst), vec!["en_US"]);
    }

    #[test]
    fn formats_keep_the_language_and_the_rest() {
        let vars = vec![
            ("LANG".to_string(), "en_US.UTF-8".to_string()),
            ("LC_MESSAGES".to_string(), "en_US.UTF-8".to_string()),
            ("LC_TIME".to_string(), "en_GB.UTF-8".to_string()),
        ];
        let (f, mixed) = formats_of(&vars);
        assert_eq!(f, "en_GB.UTF-8");
        assert!(mixed, "only LC_TIME set");
        let next = with_formats(&vars, "de_DE.UTF-8");
        assert_eq!(next[0], ("LANG".to_string(), "en_US.UTF-8".to_string()));
        assert!(next.contains(&("LC_MESSAGES".to_string(), "en_US.UTF-8".to_string())));
        for k in FORMAT_VARS {
            assert!(
                next.contains(&(k.to_string(), "de_DE.UTF-8".to_string())),
                "{k}"
            );
        }
        let (f, mixed) = formats_of(&next);
        assert_eq!((f.as_str(), mixed), ("de_DE.UTF-8", false));
        let cleared = with_formats(&next, "");
        assert_eq!(cleared.len(), 2, "LANG + LC_MESSAGES only: {cleared:?}");
        assert_eq!(formats_of(&cleared), (String::new(), false));
    }

    #[test]
    fn packs_for_a_locale() {
        let avail: BTreeSet<String> = [
            "firefox-i18n-de",
            "firefox-i18n-es-es",
            "firefox-i18n-es-mx",
            "firefox-i18n-pt-br",
            "firefox-i18n-pt-pt",
            "firefox-i18n-en-gb",
            "firefox-i18n-en-us",
            "firefox-i18n-ka",
            "firefox-i18n-ca-valencia",
            "firefox-i18n-ca",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = "firefox-i18n-";
        assert_eq!(
            pick_pack(p, "de_AT.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-de")
        );
        assert_eq!(
            pick_pack(p, "es_MX.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-es-mx")
        );
        assert_eq!(
            pick_pack(p, "es_CO.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-es-es")
        );
        assert_eq!(
            pick_pack(p, "pt_BR.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-pt-br")
        );
        assert_eq!(
            pick_pack(p, "ka_GE.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-ka")
        );
        assert_eq!(
            pick_pack(p, "en_GB.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-en-gb")
        );
        assert_eq!(pick_pack(p, "en_US.UTF-8", &avail), None);
        assert_eq!(
            pick_pack(p, "ca_ES.UTF-8", &avail).as_deref(),
            Some("firefox-i18n-ca")
        );
        assert_eq!(pick_pack(p, "xx_YY.UTF-8", &avail), None);
    }

    #[test]
    fn every_official_package_is_installable() {
        for (d, p, _) in OFFICIAL {
            assert!(dict_name_ok(d), "{d}");
            assert!(install_name_ok(p), "{p}");
        }
        for (_, d, p) in AUR {
            assert!(dict_name_ok(d), "{d}");
            assert!(install_name_ok(p), "{p}");
        }
    }
}

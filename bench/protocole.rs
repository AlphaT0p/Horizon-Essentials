// Protocole de mesure, identique avant et aprÃ¨s l'optimisation.
// N'utilise que l'API publique stable des Ã©vÃ©nements : frame, hud, tick, apply.
//
// P1 : frame + HUD en rÃ©gime Ã©tabli (2 000 000 frames) : ns et allocations par frame,
//      tÃ©moin actif puis dÃ©sactivÃ©.
// P2 : tick en rÃ©gime Ã©tabli : ns et allocations par tick.
// P3 : changement de config puis frames de 1 ms jusqu'Ã  ce que le fichier soit Ã©crit :
//      durÃ©e de la PIRE frame (c'est elle qui portait l'Ã©criture disque). 20 rÃ©pÃ©titions.
// P4 : premiÃ¨re frame d'un thread neuf, avec un fichier de config prÃ©sent (chargement) :
//      durÃ©e de cette frame. 20 rÃ©pÃ©titions.

use crate::allocs;
use crate::game::{essentials_events as ev, essentials_registry::ModuleId, essentials_types::Action};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("he-proto-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn frame_and_hud(d: &Path, sink: &mut u64) {
    ev::frame(|| d.to_path_buf(), 1.0);
    ev::hud(
        [480.0, 270.0],
        &|s: &str| s.len() as f32 * 6.0,
        &mut |_, _, _| Some([300.0, 2.0]),
        &mut |shapes, at| *sink += shapes.ops().len() as u64 + at[0] as u64,
    );
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

pub fn run() {
    let d = dir("p1");
    let mut sink = 0;
    // Chauffe : chargement fini (y compris s'il est asynchrone), formes construites.
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(300) {
        frame_and_hud(&d, &mut sink);
    }
    let n = 2_000_000u64;
    let measure = |sink: &mut u64| {
        let a0 = allocs();
        let t = Instant::now();
        for _ in 0..n {
            frame_and_hud(&d, sink);
        }
        (t.elapsed().as_nanos() as f64 / n as f64, (allocs() - a0) as f64 / n as f64)
    };
    let (on_ns, on_allocs) = measure(&mut sink);
    assert!(ev::apply(|| d.clone(), Action::Set(ModuleId::Status, false)));
    // Laisse passer la sauvegarde de ce changement avant de mesurer.
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(1500) {
        frame_and_hud(&d, &mut sink);
    }
    let (off_ns, off_allocs) = measure(&mut sink);
    let a0 = allocs();
    let t = Instant::now();
    for _ in 0..n {
        std::hint::black_box(ev::tick(|| d.clone(), true));
    }
    let tick_ns = t.elapsed().as_nanos() as f64 / n as f64;
    let tick_allocs = (allocs() - a0) as f64 / n as f64;

    // P3
    let mut worst = Vec::new();
    let file = d.join("profiles").join("default.json");
    for i in 0..20 {
        let before = std::fs::read_to_string(&file).unwrap_or_default();
        assert!(ev::apply(|| d.clone(), Action::Toggle(ModuleId::Status)));
        let mut max = Duration::ZERO;
        let start = Instant::now();
        loop {
            let t = Instant::now();
            ev::frame(|| d.clone(), 1.0);
            max = max.max(t.elapsed());
            let now = std::fs::read_to_string(&file).unwrap_or_default();
            if now != before && !now.is_empty() {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "pas sauvegardÃ© (essai {i})");
            std::thread::sleep(Duration::from_micros(200));
        }
        worst.push(max.as_secs_f64() * 1e6);
    }

    // P4
    let mut first = Vec::new();
    for _ in 0..20 {
        let d2 = d.clone();
        let us = std::thread::spawn(move || {
            let t = Instant::now();
            ev::frame(|| d2.clone(), 1.0);
            t.elapsed().as_secs_f64() * 1e6
        })
        .join()
        .unwrap();
        first.push(us);
    }

    println!("PROTOCOLE P1 frame+HUD tÃ©moin actif     : {on_ns:.1} ns/frame, {on_allocs} alloc/frame");
    println!("PROTOCOLE P1 frame+HUD tÃ©moin dÃ©sactivÃ© : {off_ns:.1} ns/frame, {off_allocs} alloc/frame");
    println!("PROTOCOLE P2 tick                       : {tick_ns:.1} ns/tick, {tick_allocs} alloc/tick");
    println!(
        "PROTOCOLE P3 pire frame pendant une sauvegarde : mÃ©diane {:.0} Âµs, max {:.0} Âµs",
        median(worst.clone()),
        worst.iter().cloned().fold(0.0, f64::max)
    );
    println!(
        "PROTOCOLE P4 premiÃ¨re frame (chargement)       : mÃ©diane {:.0} Âµs, max {:.0} Âµs",
        median(first.clone()),
        first.iter().cloned().fold(0.0, f64::max)
    );
    std::hint::black_box(sink);
    let _ = std::fs::remove_dir_all(&d);
}

use dmg_01::apu::CLOCK_RATE;
use dmg_01::audio::HostAudio;
use dmg_01::memory::MemoryBus;
use std::time::Duration;

/// Emits a quiet ~440 Hz APU pulse through the default output device.
/// Callback success is not a claim that physical speakers were audible.
#[test]
#[ignore = "requires an audio output device; emits a half-second tone"]
fn apu_host_playback_smoke() {
    let audio = HostAudio::new().expect("host audio initialization failed");
    let mut b = MemoryBus::new();
    b.apu.set_sample_rate(audio.sample_rate());
    b.write_byte(0xff26, 0x80);
    b.write_byte(0xff11, 0x80);
    b.write_byte(0xff12, 0x40);
    b.write_byte(0xff13, 0xd6);
    b.write_byte(0xff14, 0x86);
    b.write_byte(0xff24, 0x22);
    b.write_byte(0xff25, 0x11);
    for _ in 0..30 {
        b.step(70_224);
        audio.submit(b.apu.take_samples());
        std::thread::sleep(Duration::from_secs_f64(70_224.0 / f64::from(CLOCK_RATE)));
    }
    std::thread::sleep(Duration::from_millis(100));
    let frames = audio.nonzero_frames_played();
    println!(
        "host callback consumed {frames} nonzero APU frames at {} Hz",
        audio.sample_rate()
    );
    assert!(
        frames > u64::from(audio.sample_rate()) / 4,
        "audio callback did not consume the tone"
    );
}

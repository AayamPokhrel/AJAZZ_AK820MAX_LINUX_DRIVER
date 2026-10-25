use ajazz_audio::spectrum::{SpectrumAnalyzer, AUDIO_BANDS, FFT_SIZE};

#[test]
fn test_fft_pure_sine_wave() {
    let sample_rate = 48000;
    let mut analyzer = SpectrumAnalyzer::new(sample_rate, 1.0);

    // Synthesize 120 Hz pure bass tone
    let freq = 120.0f32;
    let pcm: Vec<f32> = (0..FFT_SIZE)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
        .collect();

    let levels = analyzer.update(&pcm);
    assert_eq!(levels.len(), AUDIO_BANDS);

    // Bass bands (0..=2) should have significantly higher energy than high bands (7..9)
    assert!(analyzer.bass > 0.4, "Bass level should be active, got {}", analyzer.bass);
    assert!(levels[1] > levels[8], "Band 1 (72-132 Hz) should be higher than Band 8, got {} vs {}", levels[1], levels[8]);
}

#[test]
fn test_fft_silence_gate() {
    let sample_rate = 48000;
    let mut analyzer = SpectrumAnalyzer::new(sample_rate, 1.0);

    let silence = vec![0.0f32; FFT_SIZE];
    let levels = analyzer.update(&silence);

    assert_eq!(levels.iter().sum::<f32>(), 0.0, "Silence should produce all zeros");
    assert_eq!(analyzer.energy, 0.0);
}

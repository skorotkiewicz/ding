use std::{f32::consts::TAU, io::IsTerminal, num::NonZero, process::ExitCode, time::Duration};

use clap::{Parser, ValueEnum};
use rodio::{DeviceSinkBuilder, Player, Source, buffer::SamplesBuffer};

const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const SAMPLE_RATE: u32 = 48_000;
const NOTE_GAP: usize = SAMPLE_RATE as usize / 25;
const LOOP_GAP: usize = SAMPLE_RATE as usize * 2;

#[derive(Parser)]
#[command(
    name = BIN_NAME,
    version,
    about = "Audio notifications. Loops by default; Ctrl+C stops playback.",
    after_help = "Examples:\n  wget -c http://example.com/big.zip ; ding\n  ding --preset arcade --once\n  ding --preset gentle --duration 10 --volume 0.2"
)]
struct Args {
    /// Melody to play
    #[arg(short, long, value_enum, default_value = "chime")]
    preset: Preset,

    /// Play the melody once instead of looping
    #[arg(short, long)]
    once: bool,

    /// Stop after this many seconds, including pauses; accepts decimals
    #[arg(short, long, value_parser = parse_duration, value_name = "SECONDS")]
    duration: Option<Duration>,

    /// Volume from 0 to 1
    #[arg(short, long, default_value = "0.6", value_parser = parse_volume)]
    volume: f32,

    /// List presets without opening an audio device
    #[arg(short, long)]
    list: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Preset {
    /// Bright three-note bell, the default
    Chime,
    /// Rising four-note completion tune
    Success,
    /// Quick high-pitched arcade flourish
    Arcade,
    /// Alternating urgent tones
    Alarm,
    /// Slower, softer rising melody
    Gentle,
}

impl Preset {
    fn notes(self) -> &'static [(f32, f32)] {
        match self {
            Self::Chime => &[(659.25, 0.18), (880.0, 0.18), (1318.51, 0.45)],
            Self::Success => &[
                (523.25, 0.15),
                (659.25, 0.15),
                (783.99, 0.15),
                (1046.5, 0.4),
            ],
            Self::Arcade => &[
                (880.0, 0.08),
                (1174.66, 0.08),
                (1567.98, 0.08),
                (2093.0, 0.25),
            ],
            Self::Alarm => &[(880.0, 0.25), (659.25, 0.25), (880.0, 0.25), (659.25, 0.25)],
            Self::Gentle => &[(392.0, 0.3), (523.25, 0.3), (659.25, 0.65)],
        }
    }
}

fn parse_duration(value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<f64>()
        .map_err(|_| "expected seconds, e.g. 10 or 2.5")?;
    let duration = Duration::try_from_secs_f64(seconds)
        .map_err(|_| "seconds must be positive, finite, and fit in a duration")?;
    if duration.is_zero() {
        return Err("seconds must be greater than zero".into());
    }
    Ok(duration)
}

fn parse_volume(value: &str) -> Result<f32, String> {
    let volume = value
        .parse::<f32>()
        .map_err(|_| "expected a volume from 0 to 1")?;
    if !(0.0..=1.0).contains(&volume) {
        return Err("volume must be from 0 to 1".into());
    }
    Ok(volume)
}

fn melody(preset: Preset) -> Vec<f32> {
    let mut samples = Vec::new();
    for &(frequency, seconds) in preset.notes() {
        let count = (seconds * SAMPLE_RATE as f32) as usize;
        for i in 0..count {
            let progress = i as f32 / count as f32;
            let phase = TAU * frequency * i as f32 / SAMPLE_RATE as f32;
            // Ten-millisecond attack/release avoids clicks between notes.
            let attack = (i as f32 / (SAMPLE_RATE as f32 * 0.01)).min(1.0);
            let release = ((count - 1 - i) as f32 / (SAMPLE_RATE as f32 * 0.01)).min(1.0);
            let tone = 0.8 * phase.sin() + 0.2 * (2.0 * phase).sin();
            samples.push(tone * attack * release * (-3.0 * progress).exp());
        }
        samples.resize(samples.len() + NOTE_GAP, 0.0);
    }
    samples
}

fn audio(args: &Args) -> Box<dyn Source<Item = f32> + Send> {
    let mut samples = melody(args.preset);
    if !args.once {
        samples.resize(samples.len() + LOOP_GAP, 0.0);
    }
    let buffer = SamplesBuffer::new(
        NonZero::new(1).unwrap(),
        NonZero::new(SAMPLE_RATE).unwrap(),
        samples,
    );
    let source: Box<dyn Source<Item = f32> + Send> = if args.once {
        Box::new(buffer)
    } else {
        Box::new(buffer.repeat_infinite())
    };
    match args.duration {
        Some(duration) => Box::new(source.take_duration(duration)),
        None => source,
    }
}

fn run(args: Args) -> Result<(), String> {
    if args.list {
        for preset in Preset::value_variants() {
            if let Some(value) = preset.to_possible_value() {
                println!(
                    "{:<9} {}",
                    value.get_name(),
                    value.get_help().unwrap_or_default()
                );
            }
        }
        return Ok(());
    }

    let mut output = DeviceSinkBuilder::open_default_sink().map_err(|error| {
        format!("cannot open audio output: {error}. Check your default audio device.")
    })?;
    output.log_on_drop(false);
    let player = Player::connect_new(output.mixer());
    player.set_volume(args.volume);
    player.append(audio(&args));
    if !args.once && args.duration.is_none() && std::io::stderr().is_terminal() {
        eprintln!("{BIN_NAME}: looping with a two-second pause; Ctrl+C to stop");
    }
    // Keep both the output device and player alive until playback finishes.
    player.sleep_until_end();
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{BIN_NAME}: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_and_audio() {
        assert_eq!(Args::command().get_name(), BIN_NAME);
        let defaults = Args::try_parse_from(["ding"]).unwrap();
        assert!(!defaults.once && defaults.duration.is_none());
        assert_eq!(defaults.volume, 0.6);
        for seconds in ["0", "-1", "NaN", "inf", "1e100", "nope"] {
            assert!(Args::try_parse_from(["ding", "--duration", seconds]).is_err());
        }
        for volume in ["-0.1", "1.1", "NaN", "inf", "nope"] {
            assert!(Args::try_parse_from(["ding", "--volume", volume]).is_err());
        }
        assert!(Args::try_parse_from(["ding", "--preset", "missing"]).is_err());
        assert!(Args::try_parse_from(["ding", "--list"]).unwrap().list);
        assert_eq!(parse_duration("2.5").unwrap(), Duration::from_millis(2500));
        assert_eq!(parse_volume("0").unwrap(), 0.0);
        assert_eq!(parse_volume("1").unwrap(), 1.0);

        for &preset in Preset::value_variants() {
            let samples = melody(preset);
            assert_eq!(samples[0], 0.0);
            assert_eq!(*samples.last().unwrap(), 0.0);
            assert!(samples.iter().any(|sample| sample.abs() > 0.1));
            assert!(
                samples
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
            );
            let args = Args {
                preset,
                once: true,
                ..Args::try_parse_from(["ding"]).unwrap()
            };
            assert_eq!(audio(&args).count(), samples.len());
        }

        let samples = melody(defaults.preset);
        let cycle = samples.len() + LOOP_GAP;
        let repeated: Vec<_> = audio(&defaults).take(cycle + samples.len()).collect();
        assert_eq!(&repeated[..samples.len()], samples.as_slice());
        assert!(
            repeated[samples.len()..cycle]
                .iter()
                .all(|&sample| sample == 0.0)
        );
        assert_eq!(&repeated[cycle..], samples.as_slice());

        for extra in [vec![], vec!["--once"]] {
            let args = Args::try_parse_from(["ding", "--duration", "0.1"].into_iter().chain(extra))
                .unwrap();
            assert_eq!(audio(&args).count(), SAMPLE_RATE as usize / 10);
        }
    }
}

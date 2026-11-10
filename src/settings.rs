use std::time::{Duration, Instant};

const DEFAULT_RENDER_SCALE: f32 = 0.5;
const DEFAULT_FPS: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RainType {
    Modern,
    Classic,
}

#[derive(Clone, Copy, Debug)]
pub struct Quality {
    pub fps: u32,
    pub render_scale: f32,
    pub rain_type: RainType,
    pub screensaver: bool,
}

impl Default for Quality {
    fn default() -> Self {
        Self {
            fps: DEFAULT_FPS,
            render_scale: DEFAULT_RENDER_SCALE,
            rain_type: RainType::Modern,
            screensaver: false,
        }
    }
}

impl Quality {
    pub fn from_environment() -> Result<Self, String> {
        let mut quality = Self::default();
        if let Ok(value) = std::env::var("MATRIX_FPS") {
            quality.fps = value
                .parse()
                .map_err(|_| "MATRIX_FPS must be 1–60".to_owned())?;
        }
        if let Ok(value) = std::env::var("MATRIX_RENDER_SCALE") {
            quality.render_scale = value
                .parse()
                .map_err(|_| "MATRIX_RENDER_SCALE must be 0.25–1".to_owned())?;
        }
        quality.validate()?;
        Ok(quality)
    }

    pub fn with_arguments(
        mut self,
        args: impl IntoIterator<Item = String>,
    ) -> Result<Self, String> {
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            let (flag, inline_value) = argument
                .split_once('=')
                .map_or((argument.as_str(), None), |(flag, value)| {
                    (flag, Some(value))
                });
            match flag {
                "--screensaver" if inline_value.is_none() => self.screensaver = true,
                "--type" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--type needs modern or classic")?;
                    self.rain_type = match value.as_str() {
                        "modern" => RainType::Modern,
                        "classic" => RainType::Classic,
                        _ => return Err(format!("--type must be modern or classic, got {value}")),
                    };
                }
                "--fps" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--fps needs a value from 1 to 60")?;
                    self.fps = value.parse().map_err(|_| "--fps must be 1–60")?;
                }
                "--render-scale" => {
                    let value = inline_value
                        .map(str::to_owned)
                        .or_else(|| args.next())
                        .ok_or("--render-scale needs a value from 0.25 to 1")?;
                    self.render_scale =
                        value.parse().map_err(|_| "--render-scale must be 0.25–1")?;
                }
                _ => return Err(format!("unknown argument: {argument}")),
            }
        }
        self.validate()?;
        Ok(self)
    }

    pub fn validate(self) -> Result<(), String> {
        if !(1..=60).contains(&self.fps) || !(0.25..=1.0).contains(&self.render_scale) {
            return Err("FPS must be 1–60 and render scale must be 0.25–1".to_owned());
        }
        Ok(())
    }

    pub fn render_size(self, width: u32, height: u32) -> (u32, u32) {
        (
            ((width as f32 * self.render_scale).round() as u32).max(1),
            ((height as f32 * self.render_scale).round() as u32).max(1),
        )
    }

    pub fn frame_time(self) -> Duration {
        Duration::from_secs_f64(1.0 / self.fps as f64)
    }
}

pub fn next_frame_deadline(previous: Instant, now: Instant, frame_time: Duration) -> Instant {
    let mut deadline = previous + frame_time;
    while deadline <= now {
        deadline += frame_time;
    }
    deadline
}

#![allow(unstable_name_collisions)]
use std::collections::HashMap;

use itertools::Itertools;
use nu_ansi_term::{Color, Style};
use tracing::{field::Field, Level};
use tracing_subscriber::{registry::LookupSpan, Layer};

pub struct CraftLayer;

impl<S: tracing::Subscriber + for<'lo> LookupSpan<'lo>> Layer<S> for CraftLayer {
    fn on_event(&self, event: &tracing::Event<'_>, cx: tracing_subscriber::layer::Context<'_, S>) {
        // inspiration taken from pnpm
        let level = Color::Black
            .on(match *event.metadata().level() {
                Level::INFO => Color::Green,
                Level::WARN => Color::Yellow,
                Level::ERROR => Color::Red,
                Level::DEBUG => Color::Blue,
                Level::TRACE => Color::Purple,
            })
            .bold()
            .paint(format!(" {} ", event.metadata().level().as_str()));

        let mut visitor = CraftVisitor {
            message: None,
            other_fields: HashMap::new(),
        };

        event.record(&mut visitor);

        let message = visitor
            .message
            .as_deref()
            .map(|message| format!(" {message}"))
            .unwrap_or(String::new());

        // should look like
        // field = value field2 = value2
        let other_fields = if visitor.other_fields.is_empty() {
            String::new()
        } else {
            let padding = " ".repeat(
                // length of level
                event.metadata().level().as_str().len()
                    // space after level before target
                    + 1,
            );
            let real = visitor
                .other_fields
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{} = {value}",
                        Style::default().dimmed().italic().paint(key)
                    )
                })
                .collect::<Vec<String>>()
                .join(&format!("\n{padding}"));
            format!("\n{padding}{real}",)
        };

        let target = event
            .metadata()
            .target()
            .split("::")
            .map(|module| Color::Cyan.paint(module).to_string())
            .intersperse_with(|| Color::LightRed.paint("::").to_string())
            .collect::<String>();

        println!("{level} {target}{message} {other_fields}");
    }
}

struct CraftVisitor {
    message: Option<String>,
    other_fields: HashMap<String, String>,
}

impl CraftVisitor {
    fn record(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            let _ = self.message.insert(value);
        } else {
            let _ = self.other_fields.insert(field.name().to_owned(), value);
        }
    }
}

impl tracing::field::Visit for CraftVisitor {
    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.record(field, value.to_string())
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.record(field, format!("{value:?}"))
    }
}

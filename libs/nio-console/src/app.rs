use crate::style;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_core::Stream;
use ratatui::{DefaultTerminal, prelude::*, widgets::*};
use std::{
    future::poll_fn,
    io::Result,
    pin::pin,
    task::Poll,
    time::{Duration, Instant},
};

const FPS: u64 = 24;

pub struct App {
    /// Is the application running?
    exit: bool,
    tick: nio::Interval,
    time: Instant,
    metrics: nio::metrics::RuntimeMetrics,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> App {
        App {
            time: Instant::now(),
            exit: false,
            tick: nio::interval(Duration::from_millis(1000 / FPS)),
            metrics: nio::RuntimeContext::current().metrics(),
        }
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut event_stream = pin!(EventStream::new());

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            let event = poll_fn(|cx| {
                if let Poll::Ready(val) = event_stream.as_mut().poll_next(cx) {
                    return Poll::Ready(val);
                }
                self.tick.poll(cx).map(|()| {
                    self.tick();
                    None
                })
            })
            .await;

            if let Some(Ok(ev)) = event {
                self.handle_events(ev).await?;
            }
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/master/examples>
    fn draw(&mut self, frame: &mut Frame) {
        let body = Layout::new(
            Direction::Vertical,
            [Constraint::Length(2), Constraint::Fill(1)],
        )
        .split(frame.area());

        let elapsed = self.time.elapsed();
        let frequency = self.tick.period().as_millis();

        let text = vec![
            Line::from(vec![
                Span::raw("Time: "),
                Span::styled(format!("{}", elapsed.as_secs()), style::GREEN),
                Span::styled(
                    format!(".{:03}s", elapsed.subsec_millis()),
                    style::DARK_GRAY,
                ),
            ]),
            Line::from(vec![
                Span::raw("Frequency: "),
                Span::styled(frequency.to_string(), style::YELLOW),
                Span::styled("ms; ", style::DARK_GRAY),
                Span::styled((1000 / frequency).to_string(), style::YELLOW),
                Span::styled(" FPS ", style::DARK_GRAY),
                Span::styled("(Press `+` or `-`)", style::MAGENTA),
            ]),
        ];

        frame.render_widget(Paragraph::new(text), body[0]);

        let title = Line::from("CPU Load").bold().blue().centered();
        let info = Line::from("Press `Esc`, `Ctrl-C` or `q` to stop running")
            .blue()
            .centered();

        let block = Block::bordered().title(title).title_bottom(info);

        let bar_area = block.inner(body[1]);
        frame.render_widget(block, body[1]);

        let mut bar_max_hight = 10;
        let bar_gap = 1;
        let bar_width = ((bar_area.width / self.metrics.num_workers() as u16) - bar_gap).max(5);

        let bars: Vec<_> = self
            .metrics
            .task_counts()
            .zip(style::COLORS.into_iter().cycle())
            .enumerate()
            .map(|(i, (counter, color))| {
                let value = counter.total();
                bar_max_hight = bar_max_hight.max(value.next_multiple_of(10));

                Bar::with_label(format!("C{i}"), value)
                    .style(Style::new().fg(color))
                    .value_style(style::BAR_VALUE.bg(color))
            })
            .collect();

        {
            let per_row = bars_per_row(bar_area.width, bar_width, bar_gap);
            let rows: Vec<_> = bars.chunks(per_row).collect();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Fill(1); rows.len()])
                .split(bar_area);

            for (row_bars, chunk) in rows.into_iter().zip(chunks.iter()) {
                let chart = BarChart::default()
                    .direction(Direction::Vertical)
                    .data(BarGroup::new(row_bars))
                    .max(bar_max_hight)
                    .bar_width(bar_width)
                    .bar_gap(bar_gap);

                frame.render_widget(chart, *chunk);
            }
        }
    }

    fn tick(&mut self) {}

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_events(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            (_, KeyCode::Char('+')) => {
                let frequency: u64 = self.tick.period().as_millis().try_into().unwrap();
                let step = frequency_step_amt(frequency);
                self.tick
                    .set_period(Duration::from_millis(frequency + step));
            }
            (_, KeyCode::Char('-')) => {
                let frequency: u64 = self.tick.period().as_millis().try_into().unwrap();
                if frequency <= 5 {
                    return;
                }
                let step = frequency_step_amt(frequency);
                self.tick
                    .set_period(Duration::from_millis(frequency - step));
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.exit = true;
    }
}

fn bars_per_row(area_width: u16, bar_width: u16, bar_gap: u16) -> usize {
    let unit = bar_width + bar_gap;
    (area_width / unit).max(1) as usize
}

fn frequency_step_amt(frequency: u64) -> u64 {
    match frequency {
        0..100 => 1,
        100..200 => 10,
        _ => 100,
    }
}

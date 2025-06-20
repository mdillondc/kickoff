use std::time::{Duration, Instant};
use std::{cmp, process};

use crate::calculator;
use crate::config::{Config, History};
use crate::font::Font;
use crate::selection::{Element, ElementList};
use crate::Args;
use image::{ImageBuffer, RgbaImage};
use log::{debug, error};
use nix::{
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::{fork, ForkResult},
};
use notify_rust::Notification;

pub struct App {
    pub config: Config,
    pub select_index: usize,
    pub select_input: bool,
    pub all_entries: ElementList,
    pub query: String,
    pub font: Font,
    pub history: Option<History>,
    pub last_search_result: Vec<usize>,
    pub args: Args,
    pub calculator_result: Option<(String, f64)>, // (expression, result)
}

impl App {
    pub fn new(
        args: Args,
        config: Config,
        all_entries: ElementList,
        font: Font,
        history: Option<History>,
    ) -> Self {
        let mut app = Self {
            args,
            config,
            font,
            select_index: 0,
            select_input: false,
            history,
            all_entries,
            query: String::new(),
            last_search_result: Vec::new(),
            calculator_result: None,
        };
        app.search();

        app
    }

    pub fn complete(&mut self) {
        if !self.select_input {
            let app = (*self
                .all_entries
                .as_ref_vec()
                .get(*self.last_search_result.get(self.select_index).unwrap())
                .unwrap())
            .clone();
            if self.query == app.name {
                self.select_index = if self.select_index < self.last_search_result.len() - 1 {
                    self.select_index + 1
                } else {
                    self.select_index
                };
            }
            self.query.clear();
            self.query.push_str(&app.name);
        }
    }

    pub fn nav_up(&mut self, distance: usize) {
        if self.select_index > 0 {
            self.select_index = self.select_index.saturating_sub(distance);
        } else if !self.query.is_empty() {
            self.select_input = true;
        }
    }
    
    fn get_total_results(&self) -> usize {
        let calculator_count = if self.calculator_result.is_some() { 1 } else { 0 };
        calculator_count + self.last_search_result.len()
    }

    pub fn nav_down(&mut self, distance: usize) {
        if self.select_input {
            if self.calculator_result.is_some() || !self.last_search_result.is_empty() {
                self.select_input = false;
                self.select_index = 0;
            }
        } else {
            let total_results = self.get_total_results();
            if self.select_index < total_results.saturating_sub(distance) {
                self.select_index += distance;
            }
        }
    }

    pub fn delete(&mut self) {
        self.query.pop();
        self.search();
    }

    pub fn delete_word(&mut self) {
        self.query.pop();
        loop {
            let removed_char = self.query.pop();
            if removed_char.unwrap_or(' ') == ' ' {
                break;
            }
        }
        self.search();
    }

    pub fn execute(&mut self) {
        // Check if we're selecting a calculator result
        if !self.select_input && self.calculator_result.is_some() && self.select_index == 0 {
            if let Some((_, result)) = &self.calculator_result {
                let result_str = calculator::format_result(*result);
                
                // Copy to clipboard using wl-clipboard-rs
                use wl_clipboard_rs::copy::{MimeType, Options, Source};
                let opts = Options::new();
                if let Err(e) = opts.copy(Source::Bytes(result_str.as_bytes().into()), MimeType::Text) {
                    log::error!("Failed to copy to clipboard: {}", e);
                }
                return;
            }
        }
        
        let element = if self.select_input {
            Element {
                name: self.query.to_string(),
                value: self.query.to_string(),
                base_score: 0,
            }
        } else {
            // Adjust index for calculator result
            let actual_index = if self.calculator_result.is_some() {
                if self.select_index == 0 {
                    // This should have been handled above, but just in case
                    return;
                } else {
                    self.select_index - 1
                }
            } else {
                self.select_index
            };
            
            (*self
                .all_entries
                .as_ref_vec()
                .get(*self.last_search_result.get(actual_index).unwrap())
                .unwrap())
            .clone()
        };
        if self.args.stdout {
            print!("{}", element.value);
            if let Some(mut history) = self.history.take() {
                history.inc(&element);
                history.save().unwrap();
            }
        } else {
            execute(&element, self.history.take());
        }
    }

    pub fn insert(&mut self, input: &str) {
        self.query.push_str(input);
        self.search();
    }

    pub fn search(&mut self) {
        self.last_search_result = Vec::new();
        self.calculator_result = None;
        
        // Check if query is a math expression
        if calculator::is_math_expression(&self.query) {
            if let Ok(result) = calculator::evaluate(&self.query) {
                self.calculator_result = Some((self.query.clone(), result));
            }
        }
        
        let search_results = self.all_entries.search(&self.query);

        self.select_input = false;
        self.select_index = 0;
        
        // If we have a calculator result, start with that selected
        if self.calculator_result.is_some() {
            // Calculator result will be at index 0, regular results follow
        } else if search_results.is_empty() {
            self.select_input = true;
        }

        // Build list of indices to search results
        let all_entries = self.all_entries.as_ref_vec();
        for entry in search_results {
            let index = all_entries.iter().position(|x| x == &entry);
            if let Some(i) = index {
                self.last_search_result.push(i);
            }
        }
    }

    pub fn draw(&mut self, width: u32, height: u32, scale: i32) -> RgbaImage {
        let frame_draw_start = Instant::now();
        let search_results: Vec<&Element> = self
            .last_search_result
            .iter()
            .map(|index| *self.all_entries.as_ref_vec().get(*index).unwrap())
            .collect();

        self.font.set_scale(scale);
        let padding = self.config.padding * scale as u32;
        let font_size = self.config.font_size * scale as f32;

        let mut img =
            ImageBuffer::from_pixel(width, height, self.config.colors.background.to_rgba());
        let prompt = match &self.args.prompt {
            Some(prompt) => prompt,
            None => &self.config.prompt,
        };
        let prompt_width = if prompt.is_empty() {
            0
        } else {
            let (width, _) = self.font.render(
                prompt,
                &self.config.colors.prompt,
                &mut img,
                padding,
                padding,
                None,
            );
            width + (font_size * 0.2) as u32
        };

        if !self.query.is_empty() {
            let color = if self.select_input {
                &self.config.colors.text_selected
            } else {
                &self.config.colors.text_query
            };
            self.font.render(
                &self.query,
                color,
                &mut img,
                padding + prompt_width,
                padding,
                None,
            );
        }

        let spacer = (1.5 * font_size) as u32;
        let max_entries = ((height.saturating_sub(2 * padding).saturating_sub(spacer)) as f32
            / (font_size * 1.2)) as usize;
        let offset = if self.select_index > (max_entries / 2) {
            self.select_index - max_entries / 2
        } else {
            0
        };

        let mut display_index = 0;
        
        // Display calculator result first if it exists
        if let Some((expr, result)) = &self.calculator_result {
            let result_str = calculator::format_result(*result);
            let display_text = format!("{} = {}", expr, result_str);
            let color = if display_index == self.select_index && !self.select_input {
                &self.config.colors.text_selected
            } else {
                &self.config.colors.text
            };
            self.font.render(
                &display_text,
                color,
                &mut img,
                padding,
                padding + spacer + display_index as u32 * (font_size * 1.2) as u32,
                Some((width - (padding * 2)) as usize),
            );
            display_index += 1;
        }
        
        // Display regular search results
        for (i, matched) in search_results
            .iter()
            .enumerate()
            .take(cmp::min(max_entries + offset, search_results.len()))
            .skip(offset)
        {
            if display_index >= max_entries {
                break;
            }
            
            let actual_selection_index = if self.calculator_result.is_some() {
                i + 1
            } else {
                i
            };
            
            let color = if actual_selection_index == self.select_index && !self.select_input {
                &self.config.colors.text_selected
            } else {
                &self.config.colors.text
            };
            self.font.render(
                &matched.name,
                color,
                &mut img,
                padding,
                padding + spacer + display_index as u32 * (font_size * 1.2) as u32,
                Some((width - (padding * 2)) as usize),
            );
            display_index += 1;
        }

        let elapsed = frame_draw_start.elapsed();
        debug!("frame time: {:.2?}", elapsed);

        img
    }
}

fn execute(elem: &Element, history: Option<History>) {
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            // We can't make that to long, since for some reason, even if this would be after a fork and the main programm exits,
            // wayland keeps the window alive
            std::thread::sleep(Duration::new(0, 100_000_000));
            match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive | WaitStatus::Exited(_, 0)) => {
                    if let Some(mut history) = history {
                        history.inc(elem);
                        match history.save() {
                            Ok(()) => {}
                            Err(e) => {
                                error!("{e}");
                            }
                        };
                    }
                }
                Ok(_) => {
                    /* Every non 0 statuscode holds no information since it's
                    origin can be the started application or a file not found error.
                    In either case the error has already been logged and does not
                    need to be handled here. */
                }
                Err(err) => error!("{err}"),
            }
        }

        Ok(ForkResult::Child) => {
            let err = exec::Command::new("sh").args(&["-c", &elem.value]).exec();

            // Won't be executed when exec was successful
            error!("{err}");

            Notification::new()
                .summary("Kickoff")
                .body(&format!("{err}"))
                .timeout(5000)
                .show()
                .unwrap();
            process::exit(2);
        }
        Err(e) => error!("{e}"),
    }
}

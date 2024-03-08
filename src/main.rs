use std::thread::sleep;
use std::time::{Duration, Instant};

use std::fs::File;
use std::io::BufReader;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::io::{self};

use colored::Colorize;
use rodio::{Decoder, OutputStream, Sink, Source};


fn format_time (t: Duration) -> String {
    let seconds = t.as_secs();
    let mins = seconds / 60;
    let milis = (t.as_millis() as u64 - seconds * 1000) * 10 / 100;

    if mins > 0 {
        let display_secs = seconds - mins * 60;
        format!("{:02}:{:02}.{:02}", mins, display_secs, milis)
    }
    else {
        format!("{:02}.{:02}s", seconds, milis)
    }
}

#[derive(PartialEq)]
enum PomodoroState {
    Idle,
    Running,
    PendingContinueInput
} 

fn main() {
    let esc_char = 27 as char;
    let idle_instruction = format!("Press ({}) to start focusing, ({}) to quit.{esc_char}[1;1H", "S".green(), "Q".red());
    let running_instruction = format!("Press ({}) to pause focusing, ({}) to quit.{esc_char}[1;1H", "P".green(), "Q".red());
    let continue_instruction = format!("Press ({}) to continue, Press ({}) to pause focusing, ({}) to quit.{esc_char}[1;1H", "C".blue(), "P".green(), "Q".red());
    let mut instruction = idle_instruction.clone();

    let (_stream, sound_device) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&sound_device).unwrap();
    let audio_file_motiv = BufReader::new(File::open("alarm2.mp3").unwrap());
    let audio_file_break = BufReader::new(File::open("alarm.mp3").unwrap());
    let audio_source_motiv = Decoder::new(audio_file_motiv).unwrap().repeat_infinite().buffered();
    let audio_source_break = Decoder::new(audio_file_break).unwrap().repeat_infinite().buffered();

    let _stdout = io::stdout().into_raw_mode().unwrap();
    let mut stdin = termion::async_stdin().keys();

    let mut timer_target = Instant::now() + Duration::from_secs(10);

    //states: Focus / Short Break / Long Break
    let state_cycle_word = ["Focus", "Short Break", "Focus", "Short Break", "Focus", "Short Break", "Focus", "Long Break"];
    let state_cycle_time = [1800,     300,           1800,    300,           1800,    300,           1800,    1800];
    let mut current_state_idx = 0;
    let mut pomo_state = PomodoroState::Idle;

    loop {
        let input = stdin.next();
        if let Some(Ok(key)) = input {
            match key {
                termion::event::Key::Char('q') => break,
                termion::event::Key::Char('s') => {
                    if pomo_state == PomodoroState::Idle {
                        pomo_state = PomodoroState::Running;
                        instruction = running_instruction.clone();
                        timer_target = Instant::now() + Duration::from_secs(state_cycle_time[current_state_idx]);
                        sink.skip_one();
                    }
                },
                termion::event::Key::Char('c') => {
                    if pomo_state == PomodoroState::PendingContinueInput {
                        current_state_idx += 1;
                        if current_state_idx == state_cycle_time.len() {
                            current_state_idx = 0;
                        }
                        pomo_state = PomodoroState::Running;
                        instruction = running_instruction.clone();
                        timer_target = Instant::now() + Duration::from_secs(state_cycle_time[current_state_idx]);
                        sink.skip_one();
                    }
                },
                termion::event::Key::Char('p') => {
                    pomo_state = PomodoroState::Idle;
                    instruction = idle_instruction.clone();
                    sink.skip_one();
                },
                _ => ()
            }
        }


        println!("{esc_char}[2J{esc_char}[1;1H{instruction}");
        if pomo_state != PomodoroState::Idle {
            let now = Instant::now();
            let mut duration = timer_target - now;
            let mut sign = "-";
            let mut duration_str = format_time(duration).green();

            if duration.as_millis() == 0 {
                duration = now - timer_target;
                sign = "+";
                if pomo_state != PomodoroState::PendingContinueInput {
                    pomo_state = PomodoroState::PendingContinueInput;
                    let sound_to_play = if current_state_idx % 2 != 0 {
                        audio_source_motiv.clone()
                    } else {
                        audio_source_break.clone()
                    };
                    sink.append(sound_to_play);
                }
                instruction = continue_instruction.clone();
                duration_str = format_time(duration).red();
            }

            let current_state_str = state_cycle_word[current_state_idx];
            println!("{esc_char}[2;1H{sign}{duration_str} {current_state_str}{esc_char}[2;1H");
        }

        sleep(Duration::from_millis(10));
    }
}

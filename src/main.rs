use std::thread::sleep;
use std::time::{Duration, Instant};

use std::fs::File;
use std::io::BufReader;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::io::{self};

use colored::Colorize;
use rodio::{Decoder, OutputStream, Sink, Source};


fn format_time (t: u128) -> String {
    let mut seconds = t / 1000;
    let mut mins = seconds / 60;
    let hours = mins / 60;
    let milis = (t - seconds * 1000) * 10 / 100;

    seconds -= mins * 60;
    mins -= hours * 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}.{:02}", hours, mins, seconds, milis)
    }
    else if mins > 0 {
        format!("{:02}:{:02}.{:02}", mins, seconds, milis)
    }
    else if seconds > 0 {
        format!("{:02}.{:02}s", seconds, milis)
    }
    else if milis > 0 {
        format!("{t}ms")
    }
    else { 
        "0".red().strikethrough().to_string()
    }
}

#[derive(Debug)]
struct Statistics {
    focus_time: u128,
    break_time: u128,
    skipped_focus_time: u128,
    skipped_break_time: u128,
    paused_time: u128,
    extra_focus_time: u128,
    extra_break_time: u128,
    completed_cycles: u128,
}
 
impl Statistics {
    fn new() -> Statistics {
        Statistics {
            focus_time: 0,
            break_time: 0,
            skipped_focus_time: 0,
            skipped_break_time: 0,
            paused_time: 0,
            extra_focus_time: 0,
            extra_break_time: 0,
            completed_cycles: 0
        }
    }

    fn print(&self) {
        let esc_char = 27 as char;
        println!("{esc_char}[2J");
        println!("{esc_char}[1;1H> Focus:           {}", format_time(self.focus_time).yellow().bold());
        println!("{esc_char}[2;1H> Break:           {}", format_time(self.break_time).yellow().bold());
        println!("{esc_char}[4;1H> {} Break:   {}", "Skipped".strikethrough(), format_time(self.skipped_break_time).yellow().bold());
        println!("{esc_char}[3;1H> {} Focus:   {}", "Skipped".strikethrough(), format_time(self.skipped_focus_time).yellow().bold());
        println!("{esc_char}[5;1H> {} Focus:     {}", "Extra".bright_white().bold(), format_time(self.extra_focus_time).yellow().bold());
        println!("{esc_char}[6;1H> {} Break:     {}", "Extra".bright_white().bold(), format_time(self.extra_break_time).yellow().bold());
        println!("{esc_char}[7;1H> Paused Time:     {}", format_time(self.paused_time).yellow().bold());
        println!("{esc_char}[8;1H> Full Cycles:     {}", format!("{}", self.completed_cycles).blue().bold());
        println!("{esc_char}[8;1H");
    }
}

#[derive(PartialEq)]
enum PomodoroState {
    Idle,
    Running,
    Paused,
    PendingContinueInput
} 

fn main() {
    let _stdout = io::stdout().into_raw_mode().unwrap();
    let mut stdin = termion::async_stdin().keys();

    let esc_char = 27 as char;
    let idle_instruction = format!("Press ({}) to start focusing, ({}) to quit.{esc_char}[1;1H", "S".green(), "Q".red());
    let running_instruction = format!("Press ({}) to pause, ({}) to skip, ({}) to end, ({}) to quit.{esc_char}[1;1H", "P".yellow(), "S".blue(), "E".purple(), "Q".red());
    let continue_instruction = format!("Press ({}) to continue, Press ({}) to pause, ({}) to quit.{esc_char}[1;1H", "C".blue(), "P".green(), "Q".red());
    let unpause_instruction = format!("Press ({}) to Resume, ({}) to quit.{esc_char}[1;1H", "R".blue(), "Q".red());
    let mut instruction = &idle_instruction;

    let (_stream, sound_device) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&sound_device).unwrap();
    let audio_file_motiv = BufReader::new(File::open("alarm2.mp3").unwrap());
    let audio_file_break = BufReader::new(File::open("alarm.mp3").unwrap());
    let audio_source_motiv = Decoder::new(audio_file_motiv).unwrap().repeat_infinite().buffered();
    let audio_source_break = Decoder::new(audio_file_break).unwrap().repeat_infinite().buffered();

    let mut remaining_time: i32 = 0;
    //states: Focus / Short Break / Long Break
    let phase_cycle = [
        ("Focus", 30),
        ("Short Break", 10),
        ("Focus", 30),
        ("Short Break", 10),
        ("Focus", 30),
        ("Long Break", 30),
    ];
    let mut current_phase_idx = 0;
    let mut pomo_state = PomodoroState::Idle;

    let mut stats = Statistics::new();

    let mut t = Instant::now();
    loop {
        let elapsed_in_loop = t.elapsed().as_millis() as u128;
        t = Instant::now();

        let input = stdin.next();
        if let Some(Ok(key)) = input {
            match key {
                termion::event::Key::Char('q') => { 
                    stats.print();
                    break;
                }
                termion::event::Key::Char('s') => {
                    match pomo_state {
                        PomodoroState::Idle => {
                            pomo_state = PomodoroState::Running;
                            instruction = &running_instruction;
                            remaining_time = phase_cycle[current_phase_idx].1 * 1000 * 60;
                            sink.skip_one();
                        }
                        PomodoroState::Running => {
                            current_phase_idx += 1;
                            if current_phase_idx == phase_cycle.len() {
                                current_phase_idx = 0;
                                stats.completed_cycles += 1;
                            }
                            remaining_time = phase_cycle[current_phase_idx].1 * 1000 * 60;
                            sink.skip_one();
                        }
                        _ => {}
                    }
                },
                termion::event::Key::Char('p') => {
                    match pomo_state {
                        PomodoroState::Running => {
                            pomo_state = PomodoroState::Paused;
                            instruction = &unpause_instruction;
                        }
                        PomodoroState::PendingContinueInput => {
                            pomo_state = PomodoroState::Paused;
                            instruction = &unpause_instruction;
                            sink.skip_one();
                        }
                        _ => {}
                    }
                }
                termion::event::Key::Char('r') => {
                    if pomo_state == PomodoroState::Paused {
                        if remaining_time > 0 {
                            pomo_state = PomodoroState::Running;
                            instruction = &running_instruction;
                        } else {
                            pomo_state = PomodoroState::PendingContinueInput;
                            instruction = &continue_instruction;
                        }
                    }
                }
                termion::event::Key::Char('c') => {
                    if pomo_state == PomodoroState::PendingContinueInput {
                        pomo_state = PomodoroState::Running;
                        current_phase_idx += 1;
                        if current_phase_idx == phase_cycle.len() {
                            current_phase_idx = 0;
                            stats.completed_cycles += 1;
                        }
                        instruction = &running_instruction;
                        remaining_time = phase_cycle[current_phase_idx].1 * 1000 * 60;
                        sink.skip_one();
                    }
                },
                termion::event::Key::Char('e') => {
                    if pomo_state == PomodoroState::Running || pomo_state == PomodoroState::Paused {
                        pomo_state = PomodoroState::Idle;
                        current_phase_idx = 0;
                        instruction = &idle_instruction;
                        sink.skip_one();
                    }
                },
                _ => ()
            }
        }


        println!("{esc_char}[2J{esc_char}[1;1H{instruction}");
        if pomo_state != PomodoroState::Idle && pomo_state != PomodoroState::Paused {
            // clock is ticking...
            remaining_time -= elapsed_in_loop as i32;
            let (current_phase_str, _) = phase_cycle[current_phase_idx];

            if current_phase_str == "Focus" {
                stats.focus_time += elapsed_in_loop as u128;
            } else {
                stats.break_time += elapsed_in_loop as u128;
            }

            let sign = if remaining_time < 0 { "+" } else { "-" };
            let duration_str = sign.to_owned() + &format_time(remaining_time.abs() as u128);
            let mut colored_duration_str = duration_str.green();

            if remaining_time < 0 {
                // changing state, playing sound.
                if pomo_state != PomodoroState::PendingContinueInput {
                    pomo_state = PomodoroState::PendingContinueInput;
                    let sound_to_play= if current_phase_idx % 2 != 0 {
                        audio_source_motiv.clone()
                    } else {
                        audio_source_break.clone()
                    };
                    sink.append(sound_to_play);
                }
                if current_phase_str == "Focus" {
                    stats.extra_focus_time += elapsed_in_loop as u128;
                } else {
                    stats.extra_break_time += elapsed_in_loop as u128;
                }
                instruction = &continue_instruction;
                colored_duration_str = duration_str.red();
            }


            println!("{esc_char}[2;1H{colored_duration_str} {current_phase_str}{esc_char}[2;1H");
        }

        if pomo_state == PomodoroState::Paused {
            stats.paused_time += elapsed_in_loop as u128;
            // Clock is still, just print to screen.
            let sign = if remaining_time < 0 { "+" } else { "-" };
            let duration_str = sign.to_owned() + &format_time(remaining_time.abs() as u128);
            let mut colored_duration_str = duration_str.green();
            if remaining_time < 0 {
                colored_duration_str = duration_str.red();
            }

            let (current_phase_str, _) = phase_cycle[current_phase_idx];
            println!("{esc_char}[2;1H{colored_duration_str} {current_phase_str}{esc_char}[2;1H");
        }

        sleep(Duration::from_millis(16));
    }
}

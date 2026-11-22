#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod navigation;
mod runtime;
mod shell_environment;

fn main() {
    let environment_issue = shell_environment::restore().err();
    runtime::run(environment_issue);
}

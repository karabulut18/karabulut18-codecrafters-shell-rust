use std::io::{Write};
use std::env;

use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use std::fs::OpenOptions;

use rustyline::config::{CompletionType, Config, BellStyle};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::completion::{Completer, Pair as CompletionPair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Result, Context, Helper};

const BUILTINS: &[&str] = &["echo", "exit", "type", "pwd", "cd"];

#[derive(Default)]
struct ShellHelper{
    all_commands: Vec<String>
}

// 2. Implement the Completer trait
impl Completer for ShellHelper {
    type Candidate = CompletionPair;
    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<CompletionPair>)> {
        // Find the starting position of the word currently being typed

        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map_or(0, |i| i + 1);
        
        let prefix = &line[start..pos];

        // Filter BUILTINS based on the current prefix
        let mut candidates: Vec<CompletionPair> = self.all_commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| CompletionPair {
                display: cmd.to_string(),
                replacement: format!("{} ", cmd)
            })
            .collect();
        candidates.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((start, candidates))
    }
}


// 3. Implement required Helper traits using default/empty implementations
impl Helper for ShellHelper {}
impl Hinter for ShellHelper {
    type Hint = String;
}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}

fn find_executable_in_path(name: &str) -> Option<PathBuf>
{
    // Improvement: Use `ok().and_then()` for more idiomatic Option/Result handling
    env::var("PATH").ok().and_then(|path_var| {
        for path in env::split_paths(&path_var)
        {
            let full_path = path.join(name);
            
            // 1. Check if the path points to a file
            if full_path.is_file()
            {
                // 2. Check if we have execute permission
                if let Ok(metadata) = full_path.metadata() {
                    // This checks if the executable bit is set for the current user.
                    // The standard mode constant for execute permission for the owner is 0o100.
                    // This is a simple, common way to check for execution permission.
                    if metadata.permissions().mode() & 0o111 != 0 {
                        return Some(full_path);
                    }
                }
            }
        }
        None
    })
}

// Check if a file at a given path is executable by the current user
fn is_executable(path: &PathBuf) -> bool {
    if path.is_file() {
        if let Ok(metadata) = path.metadata() {
            // Check if the executable bit is set for the owner, group, or others (0o111)
            return metadata.permissions().mode() & 0o111 != 0;
        }
    }
    false
}

// Scans the PATH environment variable and returns a Vec of all executable file names.
fn get_executables_in_path() -> Vec<String> {
    let mut executables = Vec::new();
    
    // Attempt to get the PATH environment variable
    if let Ok(path_var) = env::var("PATH") {
        
        // Iterate over all directories in PATH
        for path_dir in env::split_paths(&path_var) {
            
            // Check if the path is a directory we can read
            if let Ok(entries) = std::fs::read_dir(&path_dir) {
                
                // Iterate over every item in the directory
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    
                    // Check if the file is executable
                    if is_executable(&path) {
                        // We only care about the file name for autocompletion
                        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                            executables.push(file_name.to_string());
                        }
                    }
                }
            }
        }
    }
    executables
}


// Helper to handle output for built-in commands (echo, pwd, type)
fn handle_built_in_output(std_out_s: &str, std_out: Option<String>, std_out_append: bool, std_err_s: &str, std_err: Option<String>, std_err_append: bool) {

    if let Some(file_path) = std_out {
        // Use OpenOptions to open the file, truncating it if it exists (for the '>' operator)
        // if std_out_append is true, append the output

        match OpenOptions::new().write(true).append(std_out_append).create(true).truncate(!std_out_append).open(&file_path) {
            Ok(mut file) => {
                // Write the output string and a newline in one operation
                if !std_out_s.is_empty() {
                    if let Err(e) = writeln!(file, "{}", std_out_s) {
                        eprintln!("Error writing to file {}: {}", file_path, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error opening file {}: {}", file_path, e);
            }
        }
    } else {
        // No redirection, print to standard output
        if !std_out_s.is_empty() {
            println!("{}", std_out_s);
        }
    }

    if let Some(file_path) = std_err {
        match OpenOptions::new().write(true).append(std_err_append).create(true).truncate(!std_err_append).open(&file_path) {
            Ok(mut file) => {
                // Write the error string and a newline in one operation
                if !std_err_s.is_empty()
                {
                    if let Err(e) = writeln!(file, "{}", std_err_s) {
                        eprintln!("Error writing to file {}: {}", file_path, e);
                    }
                }
            }
            Err (e) => {
                eprintln!("Error opening file {}: {}", file_path, e);
            }
        }
    } else {
        // No redirection, print to standard error
        if !std_err_s.is_empty()
        {
            eprintln!("{}", std_err_s);
        }
    }
}

// execute function

/*// catch the output and print
fn execute(command: &str, args: &[&str], std_out: Option<String>, std_out_append: bool, std_err: Option<String>, std_err_append: bool)
{
    if find_executable_in_path(command).is_some() {

        let mut process_command = std::process::Command::new(command);
        process_command.args(args);
        if let Some(output_file) = std_out {
            match std::fs::OpenOptions::new()
                .write(true)
                .append(std_out_append)
                .create(true)
                .truncate(!std_out_append)
                .open(&output_file)
                {
                    Ok(file) =>
                    {
                        process_command.stdout(file);
                    }
                    Err(e) =>
                    {
                        eprintln!("Failed to open output file: {}", e);
                    }
                
                }
        }
        if let Some(error_file) = std_err {
            match std::fs::OpenOptions::new()
                .write(true)
                .append(std_err_append)
                .create(true)
                .truncate(!std_err_append)
                .open(&error_file)
                {
                    Ok(file) =>
                    {
                        process_command.stderr(file);
                    }
                    Err(e) =>
                    {
                        eprintln!("Failed to open error file: {}", e);
                    }
                }
        }

        match process_command.spawn() {
            Ok(mut child) => {
                // Wait for the command to finish and capture its exit status
                match child.wait() {
                    Ok(_) => {},
                    Err(e) => eprintln!("Execution error: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to execute {}: {}", command, e),
        }
    } else {
        println!("{}: command not found", command);
    }
}*/

fn change_directory(path: &str)
{
    // if it is absolute path, check if the directory is exist
    let target_path = if path == "~" {
        match env::var("HOME")
        {
            Ok(home_dir) => PathBuf::from(home_dir),
            Err(_) =>
            {
                eprintln!("cd: HOME not set");
                return;
            }
        }
    } else {
        PathBuf::from(path)
    };

    if env::set_current_dir(&target_path).is_err(){
        eprintln!("cd: {}: No such file or directory", path);
    }
}

fn arg_parse(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut quote_char = None;
    let mut escaped = false;

    for c in line.chars() {
        if escaped {
            if quote_char == Some('"')
            {
                if c == '"' || c == '\\' {
                    current_arg.push(c);
                }
                else
                {
                    // this backslash does not escape the character
                    // both added
                    current_arg.push('\\');
                    current_arg.push(c);
                }
            }
            else
            {
                current_arg.push(c);
            }
            escaped = false;
        }
        else if c == '\\' &&  quote_char != Some('\'') {
            escaped = true;
        }
        else if c == '"' || c == '\'' {
            match quote_char {
                None =>
                {
                    quote_char = Some(c);
                }
                Some(q) if q == c =>
                {
                    quote_char = None;
                }
                Some (_) => {
                    current_arg.push(c);
                }
            }
        }
        else if c.is_whitespace() && !quote_char.is_some()
        {
            if !current_arg.is_empty()
            {
                // use mem::take to efficently move the string
                args.push(std::mem::take(&mut current_arg));
            }
        }
        else
        {
            current_arg.push(c);
        }
    }

    if !current_arg.is_empty()
    {
        args.push(current_arg);
    }
    return args
}

fn run_command(input: &str){

    let commands: Vec<&str> = input.split('|').collect();

    let mut prev_output: Option<std::process::ChildStdout> = None;

    for (ith_command, cmd_string) in commands.iter().enumerate(){
        let raw_args = arg_parse(&cmd_string.trim());
        if raw_args.is_empty()
        {
            return;
        }

        let mut std_out_r_append = false;
        let mut std_err_r_append = false;

        // detect if there is a redirect option in the command
        let mut std_out_file: Option<String> = None;
        let mut std_err_file: Option<String> = None;
        let mut command_args: Vec<String> = Vec::new();
        let mut error_in_parsing = false;


        let mut i = 0;
        while i < raw_args.len()
        {
            let arg = &raw_args[i];
    
            // This block correctly identifies the redirection and consumes the token AND the filename
            if arg == ">>" || arg == "1>>" || arg == "2>>" // Check for all redirection tokens
            {
                i += 1; // Move past the redirection operator
                if i >= raw_args.len() {
                    error_in_parsing = true;                   
                    break;
                }
                
                // **CRITICAL:** Here you consume the filename, but you DO NOT push the filename
                // or the '>>' token to 'command_args'.
                if arg == "2>>" {
                     std_err_r_append = true;
                     std_err_file = Some(raw_args[i].clone());
                } else {
                     std_out_r_append = true;
                     std_out_file = Some(raw_args[i].clone());
                }
        
                i += 1; // Move past the filename for the NEXT loop iteration
            }
            else
            {
                // This is a normal command argument, so it is added.
                command_args.push(arg.clone());
                i += 1;
            }
        }

        if error_in_parsing || command_args.is_empty()
        {
            println!("error in parsing amk")
            return;
        }

        // ... (Execute logic) ...
        let is_last = ith_command == commands.len() - 1;
        
        // Pass the previous command's stdout as the current command's stdin.
        // Also, if it's not the last command, set up piping the current stdout.
        let new_prev_output = run_single_command(
            &raw_args,
            prev_output.take(), // Take the previous output (it's now consumed as stdin)
            std_out_file.clone(), // Redirects for the current command
            std_out_r_append,
            std_err_file.clone(),
            std_err_r_append,
            is_last,
        );
        
        prev_output = new_prev_output;
    }
}


fn run_single_command(
    command_args: &[String],
    stdin_pipe: Option<std::process::ChildStdout>, // The stdin for this command
    std_out_file: Option<String>,
    std_out_r_append: bool,
    std_err_file: Option<String>,
    std_err_r_append: bool,
    is_last: bool, // True if this is the last command in the pipeline
) -> Option<std::process::ChildStdout>{

    let command = command_args[0].as_str();
    // Map the rest of the arguments from &String to &str and collect them
    let parts: Vec<&str> = command_args[1..].iter().map(|s| s.as_str()).collect();
    match command
    {

        "echo" | "pwd" | "type" => {

            if !is_last {
                eprintln!("Built-in command '{}' in a pipe: Not supported.", command);
                    return None;
            }
            
            match command {
                "echo" =>
                {
                    let std_out_s = parts.join(" ");
                    let std_err_s = "";
                    handle_built_in_output(&std_out_s, std_out_file,std_out_r_append, std_err_s, std_err_file, std_err_r_append);
                }
                "pwd" =>
                {
                    if let Ok(current_dir) = env::current_dir()
                    {
                        let std_out_s = current_dir.to_str().unwrap().to_string();
                        handle_built_in_output(&std_out_s, std_out_file, std_out_r_append, "", std_err_file, std_err_r_append);
                    }
                    else
                    {
                        let std_err_s = "Failed to get current directory";
                        handle_built_in_output("", std_out_file, std_out_r_append, std_err_s,std_err_file, std_err_r_append);
                    }
                }
                "type" =>
                {
                    if let Some(arg) = parts.get(0)
                    {
                        let mut std_out_s = String::new();
                        let mut std_err_s = String::new();
                        if  matches!(*arg, "echo" | "exit" | "type" | "pwd" | "cd")
                        {
                            std_out_s = format!("{} is a shell builtin", arg)
                        }
                        else if let Some(path) = find_executable_in_path(arg)
                        {
                            std_out_s = format!("{} is {}", arg, path.display())
                        }
                        else
                        {
                            std_err_s = format!("{} not found", arg)
                        };
                        handle_built_in_output(&std_out_s, std_out_file, std_out_r_append,&std_err_s, std_err_file, std_err_r_append);
                    };
                }
                _ => {
                    return None;
                }
            }
            None
        }
        "exit" |"cd" =>
        {
            if stdin_pipe.is_none()
            {
                
                match command{
                    "cd" =>
                    {
                        if let Some(arg) = parts.get(0)
                        {
                            change_directory(arg);
                        }
                    }
                    "exit" =>
                    {
                        if let Some(arg) = parts.get(0)
                        {
                            if let Ok(exit_code) = arg.parse::<i32>()
                            {
                                std::process::exit(exit_code);
                            }
                            else
                            {
                                std::process::exit(1);
                            }
                        }
                        else
                        {
                            std::process::exit(0);
                        }
                    }
                    _ => {
                        return None;
                    }
                }   
            }
            None
        }
        _ =>
        {
            execute_piped(
                command, 
                &parts, 
                stdin_pipe, 
                std_out_file, 
                std_out_r_append, 
                std_err_file, 
                std_err_r_append,
                !is_last, // Pipe the output if it's NOT the last command
            )
        }
    }
}

// The execution function is updated to handle pipes
fn execute_piped(
    command: &str, 
    args: &[&str], 
    mut stdin_pipe: Option<std::process::ChildStdout>, // Input from previous pipe
    std_out: Option<String>, 
    std_out_append: bool, 
    std_err: Option<String>, 
    std_err_append: bool,
    create_pipe: bool, // True if output should be piped to the next command
) -> Option<std::process::ChildStdout>
{
    if find_executable_in_path(command).is_none() {
        println!("{}: command not found", command);
        return None;
    }
    
    let mut process_command = std::process::Command::new(command);
    process_command.args(args);
    
    // --- STDIN Handling (The Pipe Input) ---
    if let Some(pipe) = stdin_pipe.take() {
        // Set the current command's stdin to the previous command's stdout
        process_command.stdin(pipe);
    }

    // --- STDOUT Handling (The Pipe Output or File Redirect) ---
    let mut pipe_output = None;
    if create_pipe {
        // If we need to pipe, use Stdio::piped() to capture the output
        process_command.stdout(std::process::Stdio::piped());
    } else if let Some(output_file) = std_out {
        // Otherwise, if there is a file redirect, handle that (as you did before)
        match std::fs::OpenOptions::new()
            .write(true)
            .append(std_out_append)
            .create(true)
            .truncate(!std_out_append)
            .open(&output_file)
            {
                Ok(file) => {
                    process_command.stdout(file);
                }
                Err(e) => {
                    eprintln!("Failed to open error file: {}", e);
                    return None;
                }
            }
    }

    // --- STDERR Handling (File Redirect Only) ---
    if let Some(error_file) = std_err {
        // (Your existing error redirection logic)
        match std::fs::OpenOptions::new()
            .write(true)
            .append(std_err_append)
            .create(true)
            .truncate(!std_err_append)
            .open(&error_file)
            {
                Ok(file) => {
                    process_command.stderr(file);
                }
                Err(e) => {
                    eprintln!("Failed to open error file: {}", e);
                    return None;
                }
            }
    }

    // --- Spawn and Return Output Pipe ---
    match process_command.spawn() {
        Ok(mut child) => {
            // If output was piped, take and return the ChildStdout handle
            if create_pipe {
                pipe_output = child.stdout.take();
            }
            
            // IMPORTANT: If this is the final command (create_pipe=false),
            // you must wait for it to finish. If it's not the final command,
            // the subsequent `spawn` will implicitly wait via the pipe.
            if !create_pipe && stdin_pipe.is_none() {
                // If it's a standalone command, wait for it
                match child.wait() {
                    Ok(_) => {},
                    Err(e) => eprintln!("Execution error: {}", e),
                }
            }
            
            pipe_output
        }
        Err(e) => {
            eprintln!("Failed to execute {}: {}", command, e);
            None
        }
    }
}


fn main() -> std::result::Result<(), Box<dyn std::error::Error>> 
{
    let config = Config::builder().completion_type(CompletionType::List).bell_style(BellStyle::Audible).build();
    let mut all_commands = get_executables_in_path();

    for builtin in BUILTINS
    {
        if !all_commands.contains(&builtin.to_string())
        {
            all_commands.push(builtin.to_string());
        }
    }

    let prompt = "$ ";
    let helper = ShellHelper{ all_commands };
    let mut rl = Editor::<ShellHelper>::with_config(config)?;
    rl.set_helper(Some(helper));

    loop
    {
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                // Add command to history (Enables up/down arrows immediately)
                rl.add_history_entry(line.as_str());
                
                // Execute command
                run_command(&line);
            },
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                println!("^C");
                continue;
            },
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                println!("exit");
                break;
            },
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
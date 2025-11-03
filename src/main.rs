use std::io::{self, Write};
use std::env;
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;
use std::fs::OpenOptions;



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


// Helper to handle output for built-in commands (echo, pwd, type)
fn handle_built_in_output(std_out_s: &str, std_out: Option<String>, std_err_s: &str, std_err: Option<String>) {

        if let Some(file_path) = std_out {
            // Use OpenOptions to open the file, truncating it if it exists (for the '>' operator)
            match OpenOptions::new().write(true).create(true).truncate(true).open(&file_path) {
                Ok(mut file) => {
                    // Write the output string and a newline in one operation
                    if let Err(e) = writeln!(file, "{}", std_out_s) {
                        eprintln!("Error writing to file {}: {}", file_path, e);
                    }
                }
                Err(e) => {
                    eprintln!("Error opening file {}: {}", file_path, e);
                }
            }
        } else {
            // No redirection, print to standard output
            println!("{}", std_out_s);
        }

    if !std_err_s.is_empty(){
        if let Some(file_path) = std_err {
            match OpenOptions::new().write(true).create(true).truncate(true).open(&file_path) {
                Ok(mut file) => {
                    // Write the error string and a newline in one operation
                    if let Err(e) = writeln!(file, "{}", std_err_s) {
                        eprintln!("Error writing to file {}: {}", file_path, e);
                    }
                }
                Err (e) => {
                    eprintln!("Error opening file {}: {}", file_path, e);
                }
            }
        } else {
                // No redirection, print to standard error
                eprintln!("{}", std_err_s);
        }
    }
}

// execute function
// catch the output and print
fn execute(command: &str, args: &[&str], std_out: Option<String>, std_err: Option<String>)
{
    if find_executable_in_path(command).is_some() {

        let mut process_command = std::process::Command::new(command);
        process_command.args(args);
        if let Some(output_file) = std_out {
            match std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
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
                .create(true)
                .truncate(true)
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
}

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

fn main()
{
    loop
    {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let raw_args = arg_parse(&input.trim());
        if raw_args.is_empty()
        {
            continue;
        }

        // detect if there is a redirect option in the command
        let mut std_out_file: Option<String> = None;
        let mut std_err_file: Option<String> = None;
        let mut command_args: Vec<String> = Vec::new();
        let mut error_in_parsing = false;

        let mut i = 0;
        while i < raw_args.len()
        {
            let arg = &raw_args[i];
            if arg == ">" || arg == "1>"
            {
                i += 1;
                if i >= raw_args.len()
                {

                    eprintln!("syntax error near unexpected token `>'");
                    error_in_parsing = true;
                    break;
                }

                std_out_file = Some(raw_args[i].clone());
                i += 1;
            }
            else if arg == "2>"
            {
                i += 1;
                if i >= raw_args.len()
                {

                    eprintln!("syntax error near unexpected token `>'");
                    error_in_parsing = true;
                    break;
                }

                std_err_file = Some(raw_args[i].clone());
                i += 1;
            }
            else
            {
                command_args.push(arg.clone());
                i += 1;
            }
        }

        if error_in_parsing || command_args.is_empty()
        {
            continue;
        }


        let command = command_args[0].as_str();


        // Map the rest of the arguments from &String to &str and collect them
        let parts: Vec<&str> = command_args[1..].iter().map(|s| s.as_str()).collect();
        match command
        {
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
            "echo" =>
            {
                let std_out_s = parts.join(" ");
                let std_err_s = "";
                handle_built_in_output(&std_out_s, std_out_file, std_err_s,std_err_file);
            }
            "pwd" =>
            {
                if let Ok(current_dir) = env::current_dir()
                {
                    let std_out_s = current_dir.to_str().unwrap().to_string();
                    handle_built_in_output(&std_out_s, std_out_file, "",std_err_file);
                }
                else
                {
                    let std_err_s = "Failed to get current directory";
                    handle_built_in_output("", std_out_file, std_err_s,std_err_file);
                }
            }
            "cd" =>
            {
                if let Some(arg) = parts.get(0)
                {
                    change_directory(arg);
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
                    handle_built_in_output(&std_out_s, std_out_file, &std_err_s, std_err_file);
                };
            }
            _ =>
            {
                // parts is &[&str] which matches the execute function signature
                execute(command, &parts, std_out_file, std_err_file);
            }
        }
    }
}
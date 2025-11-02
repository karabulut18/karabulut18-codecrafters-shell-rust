use std::io::{self, Write};
use std::env;
use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;


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

// execute function
// catch the output and print
fn execute(command: &str, args: &[&str])
{
    if find_executable_in_path(command).is_some() {
        
        // FIX: Use `Command::new(cmd)` (the filename) instead of `Command::new(path)`.
        // This relies on the system's internal PATH search (which we verified worked)
        // and correctly sets argv[0] to the filename as the tester expects.
        match std::process::Command::new(command).args(args).spawn() {
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
        if c == '\\' {
            escaped = true;
            continue;
        }
        if escaped {
            current_arg.push(c);
            escaped = false;
            continue;
        }
        if c == '"' || c == '\'' {
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

        let args = arg_parse(&input.trim());
        if args.is_empty()
        {
            continue;
        }
        let command = args[0].as_str();

        // Map the rest of the arguments from &String to &str and collect them
        let parts_strs: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        let parts = parts_strs.as_slice(); // parts is &[&str]

        match command
        {
            "exit" =>
            {
                // 'parts' is &[&str], so parts.first() gives Option<&str>
                if let Some(arg) = parts.first()
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
                println!("{}", parts.join(" "));
            }
            "pwd" =>
            {
                if let Ok(current_dir) = env::current_dir()
                {
                    println!("{}", current_dir.display());
                }
                else
                {
                    eprintln!("Failed to get current directory");
                }
            }
            "cd" =>
            {
                if let Some(arg) = parts.first()
                {
                    change_directory(arg);
                }
            }
            "type" =>
            {
                if let Some(arg) = parts.first() // 'arg' is &str
                {
                    match *arg // arg is &str, so this works as before
                    {
                        "echo" | "exit" | "type" | "pwd" | "cd" => println!("{} is a shell builtin", arg),
                        _ =>
                        {
                          if let Some(path) = find_executable_in_path(arg)
                          {
                            println!("{} is {}", arg, path.display());
                          }
                          else
                          {
                            println!("{}: not found", arg);
                          }
                        }
                    }
                }
            }
            _ =>
            {
                // parts is &[&str] which matches the execute function signature
                execute(command, parts);
            }
        }
    }
}
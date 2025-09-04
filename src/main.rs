use std::{
    collections::HashMap,
    env,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use anyrun_interface::{
    Match, PluginInfo, PluginRef,
    abi_stable::{self, std_types::RVec},
};
use anyrun_provider_ipc::{Request, Response};
use clap::{Parser, Subcommand};

pub const PLUGIN_PATHS: &[&str] = &["/usr/lib/anyrun", "/etc/anyrun/plugins"];
// FIXME: These should somehow be shared reasonably between frontends and backends
pub const CONFIG_DIRS: &[&str] = &["/etc/xdg/anyrun", "/etc/anyrun"];

/// The program providing Anyrun plugin search results
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,

    plugins: Vec<PathBuf>,
    #[arg(long)]
    config_dir: Option<String>,
}

#[derive(Clone, Subcommand)]
enum Command {
    /// Start listening for connections at the provided socket path
    Socket { path: PathBuf },
    /// Connect to the specified socket
    ConnectTo { path: PathBuf },
}

enum WorkerResult {
    Quit,
    Continue,
}

struct PluginState {
    plugin: PluginRef,
    rx: Option<mpsc::Receiver<RVec<Match>>>,
}

struct State {
    // Has to be a Vec to preserve order
    plugins: Vec<PluginState>,
}

fn main() {
    let args = Args::parse();

    let user_dir = env::var("XDG_CONFIG_HOME")
        .map(|c| format!("{c}/anyrun"))
        .or_else(|_| env::var("HOME").map(|h| format!("{h}/.config/anyrun")))
        .unwrap();

    let config_dir = args.config_dir.map(Some).unwrap_or_else(|| {
        if PathBuf::from(&user_dir).exists() {
            Some(user_dir.clone())
        } else {
            CONFIG_DIRS
                .iter()
                .map(|path| path.to_string())
                .find(|path| PathBuf::from(path).exists())
        }
    });

    let mut plugin_dirs = vec![
        env::var("XDG_DATA_HOME")
            .map(|d| format!("{d}/anyrun/plugins"))
            .or_else(|_| env::var("HOME").map(|h| format!("{h}/.local/share/anyrun/plugins")))
            .unwrap(),
        format!("{user_dir}/plugins"),
    ];

    plugin_dirs.extend(PLUGIN_PATHS.iter().map(|p| p.to_string()));

    let mut state = State {
        plugins: Vec::new(),
    };

    for plugin in &args.plugins {
        let path = if plugin.is_absolute() {
            plugin.clone()
        } else {
            let Some(path) = plugin_dirs.iter().find_map(|dir| {
                let mut path = PathBuf::from(dir);
                path.extend(plugin);

                if path.exists() {
                    return Some(path);
                }

                let mut path = PathBuf::from(dir);
                path.extend(&PathBuf::from(format!(
                    "lib{}.so",
                    plugin.to_string_lossy().replace("-", "_")
                )));

                if path.exists() {
                    return Some(path);
                }

                None
            }) else {
                eprintln!(
                    "[anyrun-provider] Failed to locate library for plugin {}, not loading",
                    plugin.display()
                );
                continue;
            };

            path
        };

        let Ok(header) = abi_stable::library::lib_header_from_path(&path) else {
            eprintln!(
                "[anyrun-provider] Failed to load plugin `{}` header",
                path.display()
            );
            continue;
        };

        let Ok(plugin) = header.init_root_module::<PluginRef>() else {
            eprintln!(
                "[anyrun-provider] Failed to init plugin `{}` root module",
                path.display()
            );
            continue;
        };

        plugin.init()(
            config_dir
                .as_ref()
                .cloned()
                .unwrap_or(CONFIG_DIRS[0].to_string())
                .into(),
        );

        state.plugins.push(PluginState { plugin, rx: None });
    }

    match args.command {
        Command::Socket { path } => {
            let listener = UnixListener::bind(path).unwrap();

            while let Ok((stream, _)) = listener.accept() {
                match worker(stream, &mut state) {
                    Ok(res) => match res {
                        WorkerResult::Quit => break,
                        WorkerResult::Continue => (),
                    },
                    Err(why) => eprintln!("[anyrun-provider] Worker returned an error: {why}"),
                }
            }
        }
        Command::ConnectTo { path } => {
            let stream = UnixStream::connect(path).unwrap();

            match worker(stream, &mut state) {
                Ok(res) => match res {
                    WorkerResult::Quit => (),
                    WorkerResult::Continue => (),
                },
                Err(why) => eprintln!("[anyrun-provider] Worker returned an error: {why}"),
            }
        }
    }
}

/// Returns whether or not the provider should quit
fn worker(stream: UnixStream, state: &mut State) -> io::Result<WorkerResult> {
    stream.set_nonblocking(true)?;
    let mut stream = BufReader::new(stream);

    let mut buf = String::new();
    let mut read = move |stream: &mut BufReader<UnixStream>| -> io::Result<Request> {
        buf.clear();
        stream.read_line(&mut buf)?;

        serde_json::from_str(&buf).map_err(io::Error::other)
    };

    let send = move |stream: &mut BufReader<UnixStream>,
                     response: &Result<Response, anyrun_provider_ipc::Error>|
          -> io::Result<()> {
        let mut buf = serde_json::to_string(response).map_err(io::Error::other)?;
        buf.push('\n');
        stream.get_mut().write_all(buf.as_bytes())?;
        Ok(())
    };

    send(
        &mut stream,
        &Ok(Response::Ready {
            info: state
                .plugins
                .iter()
                .map(|plugin_state| plugin_state.plugin.info()())
                .collect(),
        }),
    )?;
    loop {
        for plugin_state in &mut state.plugins {
            if let Some(rx) = &plugin_state.rx {
                match rx.try_recv() {
                    Ok(matches) => {
                        plugin_state.rx = None;
                        send(
                            &mut stream,
                            &Ok(Response::Matches {
                                plugin: plugin_state.plugin.info()(),
                                matches,
                            }),
                        )?;
                    }
                    Err(mpsc::TryRecvError::Empty) => (),
                    Err(mpsc::TryRecvError::Disconnected) => plugin_state.rx = None,
                }
            }
        }

        match read(&mut stream) {
            Ok(request) => match request {
                Request::Reset => todo!(),
                Request::Query { text } => {
                    for plugin_state in &mut state.plugins {
                        let (tx, rx) = mpsc::channel();
                        let plugin = plugin_state.plugin;
                        let text = text.clone();

                        thread::spawn(move || {
                            let _ = tx.send(plugin.get_matches()(text.into()));
                        });

                        plugin_state.rx = Some(rx);
                    }
                }
                Request::Handle { plugin, selection } => todo!(),
                Request::Quit => {
                    return Ok(WorkerResult::Quit);
                }
            },
            Err(why) => match why.kind() {
                io::ErrorKind::WouldBlock => (),
                io::ErrorKind::ConnectionAborted => {
                    break;
                }
                _ => {
                    eprintln!("[anyrun-provider] Unexpected socket error: {why}");
                }
            },
        }
        thread::sleep(Duration::from_millis(10));
    }

    Ok(WorkerResult::Continue)
}

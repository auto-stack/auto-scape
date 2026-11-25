//! AutoScape 原生浏览器应用：单真实宿主窗口
//! （标题 AutoScape）承载七区 + guest AutoFrame。应用身份走冻结登记表
//! （`app://auto.demo.counter/` / `app://auto.demo.clock/` /
//! `app://auto.gallery/`）；收藏/配置落宿主用户配置目录（`--config-home`
//! 或 env `AUTOSCAPE_HOME` 可隔离）。外壳脚本（tick:动作）与窗口按钮/
//! 地址栏/菜单同一 on 消息边界；证据 JSON 收口后自动退出（窗口单例纪律）。
//!
//! 用法：auto-scape.exe [--evidence <out.json>]
//!   [--config-home <dir>] [--fixture-root <dir>]
//!   [--todo-project <013 根> --todo-code-root <目录> [--todo-data <既存绝对目录>]]
//!   [--hold-ms 8000] [--min-frames 8] [--await-window-close]
//!   [--script "2:open:app://auto.demo.counter/,5:click:2,9:open:Clock,13:back,
//!              17:forward,21:favadd,25:stop"]
//!   （脚本语法 tick:动作[:arg]；动作 open:<地址或应用名>/back/forward/
//!   refresh/stop/newtab/selecttab:<页签序>/closetab:<页签序>/click:<按钮序>
//!   /favadd/favdel:<收藏序>/bar/panel/hoverin:<地址>/hoverout。
//!   click 的 target=guest 声明序（Counter：0=-、1=Reset、2=+）。
//!   PLAN-1128 追加：filter:<关键词>（目录筛选）、tutorial:<身份>（教程预览）、
//!   source:<身份>:<序号>（只读源码预览）、closepreview、press:<消息>:<序号>
//!   （包视图激活，如 AddTodo）、type:<消息>:<序号>:<文本>（包视图文本输入，
//!   如 SetInput）。todo 身份 = `auto.demo.todo`。预览/目录零 guest。）
//!   窄窗基线：env `AUTO_VM_WINDOW=520x700`（常用档为默认 1280x800）。
//!   --prep-only：预构建 ui980 provider 后即退出（排窗前准备，不开窗）。
//!   --exit-on-script：脚本收口后经窗口主线程执行 close_all（真实资源终态），
//!   写终态证据再退出（真窗档用；不依赖外部关窗）。

use auto_scape::browser_shell::{
    autoscape_config_root, install_shell_host, resolve_fixture_root, with_shell_host,
    AutoscapeHost, AutoscapeWindow, ShellAction, AUTOSCAPE_FRAMES,
};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

fn main() {
    auto_lang_core::set_ui_host_install_hook(auto_ui_runtime::install_ui_host_capabilities);
    auto_ui_runtime::install_ui_host_capabilities();

    let mut evidence_path: Option<PathBuf> = None;
    let mut config_home: Option<PathBuf> = None;
    let mut fixture_root: Option<PathBuf> = None;
    let mut autoweb_root_ca: Option<Vec<u8>> = None;
    let mut hold_ms: u64 = 8000;
    let mut min_frames: usize = 8;
    let mut script_spec: Option<String> = None;
    let mut await_window_close = false;
    let mut todo_project: Option<PathBuf> = None;
    let mut todo_code_root: Option<PathBuf> = None;
    let mut todo_data: Option<PathBuf> = None;
    let mut exit_on_script = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--help" | "-h" => {
                println!(
                    "AutoScape\n\
                     Usage: auto-scape [--config-home <dir>] [--todo-data <dir>]\n\
                     The Gallery opens with Counter, Clock and Todo. Todo starts in memory unless a data root is supplied.\n\
                     Development options: --fixture-root, --todo-project, --todo-code-root, --evidence and --script."
                );
                return;
            }
            "--config-home" => config_home = args.next().map(PathBuf::from),
            "--fixture-root" => fixture_root = args.next().map(PathBuf::from),
            // PLAN-1128：原 Gallery 013 登记（轻量；运行闭包延至首次打开）。
            "--todo-project" => todo_project = args.next().map(PathBuf::from),
            "--todo-code-root" => todo_code_root = args.next().map(PathBuf::from),
            "--todo-data" => todo_data = args.next().map(PathBuf::from),
            // PLAN-1119：网络导航附加信任根（真窗档 e2e 注入测试 CA；
            // 信任仅作用于 AutoWeb 导航客户端，不碰 OS 信任库）。
            "--autoweb-root-ca" => {
                let pem_path = args
                    .next()
                    .map(PathBuf::from)
                    .expect("--autoweb-root-ca <pem>");
                autoweb_root_ca = Some(std::fs::read(&pem_path).unwrap_or_else(|e| {
                    eprintln!("auto-scape: read {}: {e}", pem_path.display());
                    std::process::exit(2);
                }));
            }
            "--evidence" => evidence_path = args.next().map(PathBuf::from),
            "--hold-ms" => hold_ms = args.next().and_then(|v| v.parse().ok()).unwrap_or(8000),
            "--min-frames" => min_frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(8),
            "--script" => script_spec = args.next(),
            "--await-window-close" => await_window_close = true,
            "--exit-on-script" => exit_on_script = true,
            "--prep-only" => {
                auto_ui_runtime::ui::ace_node_desktop::ensure_ui980_provider()
                    .expect("ui980 provider prep");
                return;
            }
            other => {
                eprintln!("auto-scape: unknown arg `{other}`");
                std::process::exit(2);
            }
        }
    }
    if exit_on_script && (script_spec.is_none() || evidence_path.is_none() || await_window_close) {
        eprintln!("auto-scape: --exit-on-script requires --script and --evidence without --await-window-close");
        std::process::exit(2);
    }

    let config_root = config_home
        .or_else(autoscape_config_root)
        .unwrap_or_else(|| {
            eprintln!("auto-scape: cannot locate the profile. Set --config-home <dir>.");
            std::process::exit(2);
        });
    let fixture_root = fixture_root
        .or_else(resolve_fixture_root)
        .unwrap_or_else(|| {
            eprintln!("auto-scape: cannot locate the Gallery samples. Set --fixture-root <dir>.");
            std::process::exit(2);
        });
    let mut host = AutoscapeHost::with_roots(fixture_root, config_root.clone());
    if let Some(pem) = autoweb_root_ca {
        let mut options = auto_ui_runtime::ui::autoweb_nav::AutowebNavOptions::default();
        options.extra_root_pem = Some(pem);
        host = host.with_autoweb_options(options);
    }
    let todo_project = todo_project.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../auto-ui/examples/ui/013-todo")
    });
    let todo_code_root = todo_code_root.unwrap_or_else(|| config_root.join("cache/todo-code"));
    host = host
        .with_todo_project(todo_project, todo_code_root, todo_data)
        .unwrap_or_else(|error| {
            eprintln!("auto-scape: {error}");
            std::process::exit(2);
        });
    if let Some(spec) = &script_spec {
        host.install_script(parse_script(spec));
    }
    install_shell_host(host);

    if exit_on_script {
        let observer_path = evidence_path
            .clone()
            .expect("script evidence path was validated");
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(hold_ms.max(1000));
            let script_completed = loop {
                let settled = with_shell_host(|h| h.script_done()).unwrap_or(false);
                let frames = AUTOSCAPE_FRAMES.load(Ordering::Relaxed);
                if settled && frames >= min_frames {
                    break true;
                }
                if Instant::now() >= deadline {
                    break false;
                }
                std::thread::sleep(Duration::from_millis(40));
            };
            let before = with_shell_host(|h| h.evidence())
                .unwrap_or_else(|| serde_json::json!({"error": "autoscape host gone"}));
            // The event loop owns shutdown and the observer only requests it.
            with_shell_host(|host| host.request_shutdown());
            let cleanup_deadline = Instant::now() + Duration::from_secs(15);
            while !with_shell_host(|host| host.shutdown_done()).unwrap_or(true)
                && Instant::now() < cleanup_deadline
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            let (mut json, cleanup) =
                with_shell_host(|host| (host.evidence(), host.shutdown_result().cloned()))
                    .unwrap_or_else(|| (serde_json::json!({}), None));
            json["before_window_close"] = before;
            json["script_completed"] = serde_json::json!(script_completed);
            json["shutdown"] = serde_json::json!({
                "done": cleanup.is_some(),
                "cleanup_error": cleanup.as_ref().and_then(|result| result.as_ref().err()),
            });
            json["frames_rendered"] = serde_json::json!(AUTOSCAPE_FRAMES.load(Ordering::Relaxed));
            write_evidence(&observer_path, &json);
            std::process::exit(if !matches!(cleanup, Some(Ok(()))) {
                8
            } else if !script_completed {
                9
            } else {
                0
            });
        });
    }

    if let Err(e) =
        auto_ui_runtime::ui::iced::run_app_with_title::<AutoscapeWindow>(Some("AutoScape"))
    {
        eprintln!("auto-scape: window run failed: {e:?}");
        std::process::exit(7);
    }

    let outcome = with_shell_host(|h| {
        let before = h.evidence();
        let cleanup = h.close_all();
        (before, h.evidence(), cleanup)
    });
    let (before, mut json, cleanup) = outcome.expect("AutoScape host available at window shutdown");
    json["before_window_close"] = before;
    json["frames_rendered"] = serde_json::json!(AUTOSCAPE_FRAMES.load(Ordering::Relaxed));
    json["cleanup_error"] = serde_json::json!(cleanup.as_ref().err());
    if let Some(path) = evidence_path {
        write_evidence(&path, &json);
    }
    if cleanup.is_err() {
        std::process::exit(8);
    }
}

fn write_evidence(path: &std::path::Path, json: &serde_json::Value) {
    let bytes = serde_json::to_vec_pretty(json).expect("serialize AutoScape evidence");
    if let Err(error) = std::fs::write(path, bytes) {
        eprintln!("auto-scape: cannot write {}: {error}", path.display());
        std::process::exit(10);
    }
}

fn parse_script(spec: &str) -> Vec<(usize, ShellAction)> {
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut fields = part.split(':');
        let Some(tick) = fields.next().and_then(|t| t.parse::<usize>().ok()) else {
            eprintln!("auto-scape: bad script entry `{part}`");
            std::process::exit(2);
        };
        let action = fields.next().unwrap_or("");
        let rest: Vec<&str> = fields.collect();
        let step = match action {
            "open" => {
                if rest.is_empty() {
                    eprintln!("auto-scape: open needs an address: {part}");
                    std::process::exit(2);
                }
                // 地址可含 ":"（app://…）——按原样重组。
                ShellAction::Navigate(rest.join(":"))
            }
            "back" => ShellAction::Back,
            "forward" => ShellAction::Forward,
            "refresh" => ShellAction::Refresh,
            "stop" => ShellAction::Stop,
            "newtab" => ShellAction::NewTab,
            "selecttab" | "closetab" | "click" | "favdel" => {
                let Some(value) = rest.first().and_then(|v| v.parse::<usize>().ok()) else {
                    eprintln!("auto-scape: {action} needs a number: {part}");
                    std::process::exit(2);
                };
                match action {
                    "selecttab" => ShellAction::SelectTab(value),
                    "closetab" => ShellAction::CloseTab(value),
                    "click" => ShellAction::Click(value),
                    _ => ShellAction::FavoriteDelete(value),
                }
            }
            "favadd" => ShellAction::FavoriteAdd,
            "favopen" => {
                let Some(index) = rest.first().and_then(|v| v.parse::<usize>().ok()) else {
                    eprintln!("auto-scape: favopen needs a bookmark index: {part}");
                    std::process::exit(2);
                };
                ShellAction::FavoriteOpen(index)
            }
            "bar" => ShellAction::ToggleBar,
            "panel" => ShellAction::TogglePanel,
            "hoverin" => {
                if rest.is_empty() {
                    eprintln!("auto-scape: hoverin needs an address: {part}");
                    std::process::exit(2);
                }
                ShellAction::HoverIn(rest.join(":"))
            }
            "hoverout" => ShellAction::HoverOut,
            // PLAN-1128：目录筛选 / 教程与只读源码预览 / 包视图输入。
            "filter" => ShellAction::CatalogFilter(rest.join(":")),
            "tutorial" => ShellAction::PreviewTutorial(rest.join(":")),
            "source" => {
                let Some(identity) = rest.first() else {
                    eprintln!("auto-scape: source needs <identity>:<index>: {part}");
                    std::process::exit(2);
                };
                let Some(index) = rest.get(1).and_then(|v| v.parse::<usize>().ok()) else {
                    eprintln!("auto-scape: source needs a source index: {part}");
                    std::process::exit(2);
                };
                ShellAction::PreviewSource((*identity).to_string(), index)
            }
            "closepreview" => ShellAction::ClosePreview,
            "baropen" => ShellAction::BarOpen(rest.join(":")),
            "press" | "type" => {
                let Some(message) = rest.first() else {
                    eprintln!("auto-scape: {action} needs a package message: {part}");
                    std::process::exit(2);
                };
                let Some(occurrence) = rest.get(1).and_then(|v| v.parse::<usize>().ok()) else {
                    eprintln!("auto-scape: {action} needs a zero-based occurrence: {part}");
                    std::process::exit(2);
                };
                let text = (action == "type").then(|| rest[2..].join(":"));
                ShellAction::PackageInput {
                    message: (*message).to_string(),
                    occurrence,
                    text,
                }
            }
            other => {
                eprintln!("auto-scape: unknown script action `{other}`");
                std::process::exit(2);
            }
        };
        out.push((tick, step));
    }
    out
}

//! PLAN-1128 AutoScape 原生 Gallery 首批整合永久 case（AutoScape 应用）。
//!
//! 覆盖：V-01（七区外壳经统一导航打开原 013 Todo，真实包视图文本输入→新增→
//! 受检 Store→原生帧回读，关窗后 `todo.json` 与资源终态；Counter 同窗切换与
//! 两页签跨进程重开由真窗 B3/C3 覆盖，CPU 新用例保持单次 Todo 编译 ≤10 s）、
//! V-02（目录/搜索/预览零 guest/零 Todo 闭包准备；预览只读其局部资料；首次
//! 打开才准备）、V-03（显式数据根合法快照的读取路径：新宿主读回行与 ID）、
//! V-04（教程/只读源码按版本与摘要追溯；错修订/篡改具名拒绝，不冒充当前版本）。
//!
//! 启动前按应用 README 预构建 ui980 provider。

#![cfg(windows)]

use auto_scape::browser_shell::{AutoscapeHost, SHELL_IDENTITY_TODO};
use auto_ui_runtime::ui::ace_node_desktop::{current_session, StepOutcome};
use auto_ui_runtime::ui::ace_state_bridge::AceStateValue;
use auto_ui_runtime::ui::ase_gallery::{self, PreviewKind, TODO_REVISION, TODO_VERSION};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../auto-ui")
}

fn fixture_root() -> PathBuf {
    match std::env::var("ASE_M0_TEXT_ROOT") {
        Ok(root) => PathBuf::from(root),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../auto-lang/experimental/ase-m0/fixtures"),
    }
}

fn todo_project() -> PathBuf {
    repo_root().join("examples/ui/013-todo")
}

fn scratch(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("p1128-{tag}-"))
        .tempdir()
        .expect("scratch dir")
}

fn host_with_todo(dir: &Path, data_root: Option<PathBuf>) -> AutoscapeHost {
    AutoscapeHost::with_roots(fixture_root(), dir.join("config"))
        .with_todo_project(todo_project(), dir.join("code"), data_root)
        .expect("register Gallery 013")
}

fn step(host: &mut AutoscapeHost) {
    match host.step() {
        StepOutcome::DispatchRefused(error) | StepOutcome::LaneError(error) => panic!("{error}"),
        _ => {}
    }
}

fn until(host: &mut AutoscapeHost, what: &str, mut done: impl FnMut(&AutoscapeHost) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done(host) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}: {}",
            host.evidence()
        );
        step(host);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn visible() -> String {
    current_session(|session| {
        session
            .package_frame(0)
            .map(|frame| format!("{:?}", frame.view))
    })
    .flatten()
    .unwrap_or_default()
}

fn engine(host: &AutoscapeHost, session: usize, name: &str) -> Option<AceStateValue> {
    host.session_host()
        .engine_states(session)?
        .into_iter()
        .find_map(|(field, value)| (field == name).then_some(value))
}

fn active_session(host: &AutoscapeHost) -> Option<usize> {
    host.tabs()
        .get(host.active_index())
        .and_then(|tab| tab.committed.as_ref())
        .and_then(|page| page.session)
}

fn committed_address(host: &AutoscapeHost) -> Option<String> {
    host.tabs()
        .get(host.active_index())
        .and_then(|tab| tab.committed_address().map(str::to_string))
}

fn open_todo(host: &mut AutoscapeHost) -> usize {
    host.navigate("Todo")
        .expect("Todo resolves through the catalog");
    until(host, "Todo commit + first real frame", |host| {
        active_session(host).is_some() && !visible().is_empty()
    });
    assert_eq!(
        committed_address(host).as_deref(),
        Some("app://auto.demo.todo/")
    );
    active_session(host).expect("committed Todo session")
}

/// V-02/V-04：目录、搜索、预览零 guest/零 Todo 闭包准备；预览按登记摘要
/// 只读其局部资料；错修订/缺失具名拒绝。
#[test]
fn p1128_v02_catalog_search_and_previews_are_zero_load() {
    let dir = scratch("catalog");
    let dir = dir.path();
    let mut host = host_with_todo(dir, None);

    // 三条登记：Counter / Clock / Todo（声明序），Todo 带冻结版本与修订。
    let identities: Vec<&str> = host
        .catalog()
        .iter()
        .map(|entry| entry.identity.as_str())
        .collect();
    assert_eq!(
        identities,
        vec!["auto.demo.counter", "auto.demo.clock", SHELL_IDENTITY_TODO]
    );
    let todo = host
        .catalog()
        .iter()
        .find(|entry| entry.identity == SHELL_IDENTITY_TODO)
        .expect("Todo catalog entry")
        .clone();
    assert_eq!(todo.version, TODO_VERSION);
    assert_eq!(todo.revision, TODO_REVISION);
    assert!(
        todo.tutorial.is_some(),
        "Todo registers the original tutorial"
    );
    assert!(todo
        .sources
        .iter()
        .any(|doc| doc.path == "src/front/todo_store.at"));
    assert!(todo.sources.iter().any(|doc| doc.path == "src/back/db.at"));

    // 目录/搜索零装载、零准备、零读取。
    assert_eq!(host.session_host().metrics.loads, 0);
    assert!(!host.session_host().todo_prepared());
    assert_eq!(host.catalog_read_meter(), (0, 0));
    host.set_catalog_filter("todo".into());
    assert_eq!(host.filtered_catalog().len(), 1);
    host.set_catalog_filter("示例".into());
    assert_eq!(host.filtered_catalog().len(), 2);
    host.set_catalog_filter("不存在".into());
    assert!(host.filtered_catalog().is_empty());
    host.set_catalog_filter(String::new());
    assert_eq!(host.filtered_catalog().len(), 3);
    assert_eq!(host.session_host().metrics.loads, 0);
    assert!(!host.session_host().todo_prepared());
    assert_eq!(host.catalog_read_meter(), (0, 0));

    // 教程预览：只读 README，版本/修订与登记一致，正文覆盖本次真实操作。
    host.open_preview(SHELL_IDENTITY_TODO, PreviewKind::Tutorial)
        .expect("tutorial preview");
    let preview = host
        .active_preview()
        .expect("active tutorial preview")
        .clone();
    assert_eq!(preview.version, TODO_VERSION);
    assert_eq!(preview.revision, TODO_REVISION);
    assert_eq!(preview.path, "README.md");
    assert!(preview.body.contains("TodoMVC"));
    assert!(preview.body.contains("Add todo"));
    assert!(preview.bytes > 0);
    assert_eq!(host.session_host().metrics.loads, 0);
    assert!(!host.session_host().todo_prepared());

    // 只读源码预览：至少一份实际运行的 Store 与后端源。
    host.open_preview(SHELL_IDENTITY_TODO, PreviewKind::Source(1))
        .expect("store source preview");
    assert_eq!(
        host.active_preview().expect("store preview").path,
        "src/front/todo_store.at"
    );
    host.open_preview(SHELL_IDENTITY_TODO, PreviewKind::Source(2))
        .expect("backend source preview");
    assert_eq!(
        host.active_preview().expect("backend preview").path,
        "src/back/db.at"
    );
    assert_eq!(host.session_host().metrics.loads, 0);

    // 越界索引具名拒绝（不动已提交页面，不冒充内容）。
    let error = host
        .open_preview(SHELL_IDENTITY_TODO, PreviewKind::Source(99))
        .expect_err("out-of-range source must be refused");
    assert!(error.contains("越界"), "{error}");
    assert!(host.preview_error().is_some());

    // 错修订/被篡改的教程：复制 013 后改写教程，按登记摘要核验必须拒绝。
    let copy = dir.join("tampered");
    std::fs::create_dir_all(copy.join("src/front")).unwrap();
    std::fs::create_dir_all(copy.join("src/back")).unwrap();
    std::fs::copy(todo_project().join("pac.at"), copy.join("pac.at")).unwrap();
    for file in ["app.at", "todo_list.at", "todo_store.at", "types.at"] {
        std::fs::copy(
            todo_project().join("src/front").join(file),
            copy.join("src/front").join(file),
        )
        .unwrap();
    }
    for file in ["api.at", "db.at"] {
        std::fs::copy(
            todo_project().join("src/back").join(file),
            copy.join("src/back").join(file),
        )
        .unwrap();
    }
    std::fs::write(copy.join("README.md"), "# 被替换的教程\n\n不是登记修订。\n").unwrap();
    let tampered = ase_gallery::todo_entry(&copy);
    let error = ase_gallery::open_preview(&tampered, PreviewKind::Tutorial)
        .expect_err("tampered tutorial must be refused");
    assert!(error.contains("摘要不符"), "{error}");
    let missing = ase_gallery::todo_entry(&copy.join("missing-root"));
    let error = ase_gallery::open_preview(&missing, PreviewKind::Source(0))
        .expect_err("missing source must be refused");
    assert!(error.contains("预览不可用"), "{error}");

    // 一次真实预览读取后计量增长；目录/搜索从未触发准备或装载。
    let (reads, bytes) = host.catalog_read_meter();
    assert!(reads >= 3 && bytes > 0, "preview reads {reads}/{bytes}");
    assert_eq!(host.session_host().metrics.loads, 0);
    assert!(!host.session_host().todo_prepared());
    host.close_all().unwrap();
}

/// V-01：七区统一导航打开真实 013，包视图文本输入→新增→受检 Store→原生帧；
/// 关窗后显式数据根的 `todo.json` 提交与资源终态。单次 Todo 编译（≤10 s 余量；
/// Counter 切换与跨进程重开分别由真窗 B3/C3 与下方恢复 case 承担）。
#[test]
fn p1128_v01_todo_input_persists_and_releases() {
    assert!(
        std::env::var_os("ACE_NODE_DESKTOP_PREPARED_RUSTC").is_some(),
        "prepare ui980 with autoscape_window --prep-only and set \
         ACE_NODE_DESKTOP_PREPARED_RUSTC to rustc --version",
    );
    let dir = scratch("todo");
    let dir = dir.path();
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let mut host = host_with_todo(dir, Some(data.clone()));

    // 目录阶段仍未准备闭包。
    assert!(!host.session_host().todo_prepared());
    let first = open_todo(&mut host);
    assert!(host.session_host().todo_prepared());
    assert_eq!(host.session_host().metrics.loads, 1);
    // 首帧只证明已挂载；真实列表来自受检 Store 的首次读取。
    until(&mut host, "seed rows from the checked Store", |_| {
        visible().contains("Learn Auto Language")
    });
    let frame = visible();
    assert!(frame.contains("Learn Auto Language"), "{frame}");
    assert_eq!(
        engine(&host, first, "st_active_count"),
        Some(AceStateValue::Int(3))
    );
    // 真实包视图（不是文字投影）已可用：输入/按钮走受检投影同一队列。
    assert_eq!(host.evidence()["package_view"], serde_json::json!(true));

    // 包视图文本输入 + 新增按钮（消息面与窗口 on 边界同口）。
    host.package_input("SetInput", 0, Some("PLAN-1128 原生 Gallery ✓".into()))
        .unwrap();
    step(&mut host);
    host.package_input("AddTodo", 0, None).unwrap();
    until(&mut host, "Todo add committed", |host| {
        engine(host, first, "st_active_count") == Some(AceStateValue::Int(4))
            && visible().contains("PLAN-1128 原生 Gallery ✓")
    });
    assert!(visible().contains("PLAN-1128 原生 Gallery ✓"));
    assert_eq!(
        host.session_host().todo_inventory().unwrap().durable_writes,
        1
    );

    host.close_all().unwrap();
    let terminal = host.session_host().todo_inventory().unwrap();
    assert_eq!(
        (terminal.leases, terminal.workers, terminal.owners),
        (0, 0, 0)
    );
    let persisted: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(data.join("todo.json")).expect("todo.json committed"),
    )
    .expect("todo.json parses");
    assert!(persisted.to_string().contains("PLAN-1128 原生 Gallery ✓"));
    assert_eq!(persisted["next_id"], serde_json::json!(5));
    drop(host);
}

/// V-03：显式数据根中的合法快照（版本 1，行 + next_id）由新宿主读回同一
/// 提交，帧显示快照行且读取不产生写入。单次 Todo 编译。
#[test]
fn p1128_v03_seeded_snapshot_reopens_with_rows() {
    assert!(
        std::env::var_os("ACE_NODE_DESKTOP_PREPARED_RUSTC").is_some(),
        "prepare ui980 with autoscape_window --prep-only and set \
         ACE_NODE_DESKTOP_PREPARED_RUSTC to rustc --version",
    );
    let dir = scratch("restore");
    let dir = dir.path();
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let snapshot = serde_json::json!({
        "version": 1,
        // 快照校验要求 next_id >= 4（004 seed 下界）；行 id 升序且 < next_id。
        "next_id": 4,
        "todos": [
            {"id": 0, "text": "快照恢复甲", "done": false},
            {"id": 1, "text": "快照恢复乙 ✓", "done": true},
        ],
    });
    std::fs::write(data.join("todo.json"), snapshot.to_string()).unwrap();

    let mut host = host_with_todo(dir, Some(data));
    let session = open_todo(&mut host);
    until(&mut host, "snapshot rows on reopen", |_| {
        visible().contains("快照恢复甲") && visible().contains("快照恢复乙 ✓")
    });
    assert_eq!(
        engine(&host, session, "st_active_count"),
        Some(AceStateValue::Int(1))
    );
    assert_eq!(
        host.session_host().todo_inventory().unwrap().durable_writes,
        0
    );
    host.close_all().unwrap();
    let terminal = host.session_host().todo_inventory().unwrap();
    assert_eq!(
        (terminal.leases, terminal.workers, terminal.owners),
        (0, 0, 0)
    );
    drop(host);
}

/// V-04 负控：闭包被改动后首次导航不把新闭包装成登记修订（具名拒载、零会话）。
#[test]
fn p1128_v04_drifted_closure_is_refused_before_any_session() {
    assert!(
        std::env::var_os("ACE_NODE_DESKTOP_PREPARED_RUSTC").is_some(),
        "prepare ui980 with autoscape_window --prep-only and set \
         ACE_NODE_DESKTOP_PREPARED_RUSTC to rustc --version",
    );
    let dir = scratch("drift");
    let dir = dir.path();
    let copy = dir.join("project");
    copy_project(&todo_project(), &copy);
    // 一个业务源的增量修改 = 未登记的新修订。
    let app = copy.join("src/front/app.at");
    let mut source = std::fs::read_to_string(&app).unwrap();
    source.push_str("\n// PLAN-1128 未登记修订\n");
    std::fs::write(&app, source).unwrap();

    let mut host = AutoscapeHost::with_roots(fixture_root(), dir.join("config"))
        .with_todo_project(copy, dir.join("code"), None)
        .expect("register the modified project");
    let error = host
        .navigate("Todo")
        .expect_err("drifted closure must be refused");
    assert!(error.contains("source revision mismatch"), "{error}");
    assert!(host.session_host().live_ids().is_empty());
    assert!(active_session(&host).is_none());
    host.close_all().unwrap();
}

/// 复制原 013 的运行闭包与教程（不复制 gen/node_modules 等生成物）。
fn copy_project(from: &Path, to: &Path) {
    for dir in ["src/front", "src/back"] {
        std::fs::create_dir_all(to.join(dir)).unwrap();
        for entry in std::fs::read_dir(from.join(dir)).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().is_some_and(|ext| ext == "at") {
                std::fs::copy(entry.path(), to.join(dir).join(entry.file_name())).unwrap();
            }
        }
    }
    std::fs::copy(from.join("pac.at"), to.join("pac.at")).unwrap();
    std::fs::copy(from.join("README.md"), to.join("README.md")).unwrap();
}

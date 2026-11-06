"""Capture the tracked source and workspace inventory for the WireTerm P0 refactor."""

import json
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "refactor"
OUT.mkdir(parents=True, exist_ok=True)


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT).decode("utf-8")


def capture():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
    ))
    names = {p["name"] for p in metadata["packages"]}
    packages = []
    for p in sorted(metadata["packages"], key=lambda p: p["name"]):
        packages.append({
            "name": p["name"],
            "path": Path(p["manifest_path"]).relative_to(ROOT).as_posix(),
            "dependencies": sorted({d["name"] for d in p["dependencies"]}),
            "workspace_dependencies": sorted({d["name"] for d in p["dependencies"]} & names),
            "reverse_dependencies": sorted({q["name"] for q in metadata["packages"]
                if any(d["name"] == p["name"] for d in q["dependencies"])}),
        })
    files = git("ls-files").splitlines()
    rust = [f for f in files if f.endswith(".rs") and (ROOT / f).exists()]
    exe = ROOT / "target" / "debug" / "warp-oss.exe"
    return {
        "sha": git("rev-parse", "HEAD").strip(),
        "platform": sys.platform,
        "rustc": subprocess.check_output(["rustc", "--version"]).decode().strip(),
        "tracked_files": sum((ROOT / f).is_file() for f in files),
        "rust_files": len(rust),
        "rust_lines": sum(len((ROOT / f).read_bytes().splitlines()) for f in rust),
        "rust_bytes": sum((ROOT / f).stat().st_size for f in rust),
        "workspace_crates": len(packages),
        "lock_packages": len(tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]),
        "workspace_dependency_declarations": len(tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["dependencies"]),
        "executable_bytes": exe.stat().st_size if exe.exists() else None,
        "packages": packages,
    }


def write_inventory(data):
    retained = {
        "warpui", "warpui_core", "warpui_extras", "warp_core", "warp_terminal", "warp_editor",
        "sum_tree", "string-offset", "command", "settings", "settings_value", "settings_value_derive",
        "warp_logging", "warp_errors", "persistence", "warp_util", "warp_assets", "asset_macro",
        "asset_cache", "channel_versions", "fuzzy_match", "markdown_parser", "regex_dfas", "vim",
        "watcher", "prevent_sleep", "ui_components", "syntax_tree", "languages", "warp_files",
    }
    candidates = {
        "warp_tui", "voice_input", "warp_server_auth", "warp_server_client", "warp_multi_agent_client",
        "cloud_objects", "cloud_object_client", "cloud_object_models", "cloud_object_persistence",
        "firebase", "warp_graphql", "warp_graphql_schema", "warp_harness_usage", "onboarding",
    }
    lines = ["# 模块清单", "", f"原始 SHA：`{data['sha']}`。完整原始数据见 baseline.json。", "",
        "保留指保护通用能力；删除候选必须先解除反向引用；替换指混合业务模块；待确认不授权整块删除。", "",
        "## Cargo workspace", "", "| Crate | Manifest | 处置 | 直接依赖数 |", "|---|---|---|---:|"]
    deps = ["# 依赖分析", "", "## Workspace 正向和反向依赖", "",
        "以下为 Cargo metadata 声明图，包含 target、optional、dev/build 依赖，不等于 Windows GUI 激活图。", "",
        "| Crate | Workspace 直接依赖 | 被引用方 |", "|---|---|---|"]
    for p in data["packages"]:
        disposition = "保留" if p["name"] in retained else "删除候选（先解耦）" if p["name"] in candidates else "替换/待确认"
        lines.append(f"| {p['name']} | `{p['path']}` | {disposition} | {len(p['dependencies'])} |")
        deps.append(f"| {p['name']} | {', '.join(p['workspace_dependencies']) or '—'} | {', '.join(p['reverse_dependencies']) or '—'} |")
    lines += ["", "## 应用业务模块", "", "| 路径 | 处置 |", "|---|---|"]
    protected = {"terminal", "pane_group", "tab", "workspace", "app_state", "appearance", "themes", "keyboard",
        "persistence", "user_config", "launch_configs", "tab_configs", "platform", "util", "view_components"}
    deletion = {"drive", "auth", "billing", "pricing", "tracing", "autoupdate", "voice", "tui", "tui_export",
        "tui_onboarding_markers", "tui_test_support", "workspaces", "server", "referral_theme_status", "reward_view"}
    for path in sorted((ROOT / "app/src").iterdir()):
        if path.name.endswith("_tests.rs"):
            continue
        name = path.stem if path.is_file() else path.name
        disposition = "保留" if name in protected else "删除候选（检查调用链）" if name in deletion else "替换/待确认"
        lines.append(f"| `{path.relative_to(ROOT).as_posix()}` | {disposition} |")
    (OUT / "module-inventory.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    (OUT / "dependency-analysis.md").write_text("\n".join(deps) + "\n", encoding="utf-8")


if __name__ == "__main__":
    label = sys.argv[1]
    data = capture()
    (OUT / f"{label}.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    if label == "baseline":
        write_inventory(data)
    print(json.dumps({k: v for k, v in data.items() if k != "packages"}, indent=2))

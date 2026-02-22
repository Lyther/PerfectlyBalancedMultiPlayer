# Stellaris Multiplayer Mod Sync (SMMS) — High-Level Design

> **Version:** 0.1-draft
> **Date:** 2026-02-22
> **Author:** Catherine Li
> **Maintainer Note (2026-02-22):** This file includes both historical alternatives and a later superseding appendix. For implementation alignment, treat the appendix "Review" + "No-Bullshit Edition" sections as authoritative.

---

## 1. Problem Statement

Stellaris 的联机 mod 同步机制极其脆弱。核心问题有三层：

1. **Checksum 不一致**：Stellaris 通过 `checksum_manifest.txt` 定义哪些文件夹参与校验（`common/`, `events/`, `map/`, `localisation_synced/` 等）。任何玩家的这些文件夹内容存在哪怕 1 byte 差异，checksum 就不同，无法进入同一房间。
2. **Steam Workshop 版本漂移**：Workshop mod 的更新时机不可控——mod 作者推送更新后，各玩家的 Steam 客户端不一定同时下载，导致同一 mod 的文件内容在不同机器上暂时不一致。
3. **月度不同步弹窗**：即使通过 checksum patcher 绕过校验进入游戏，mod 文件的细微差异（特别是脚本逻辑差异）会导致游戏状态分叉，引发每月一次的 desync 错误。

**根因**：没有一个机制能保证所有玩家机器上参与游戏运算的文件 bit-for-bit 完全一致。

---

## 2. Existing Solutions & Gap Analysis

| 工具 | 做了什么 | 没做什么 |
|------|---------|---------|
| **Steam Workshop Collection** | 统一订阅列表 | 不保证版本一致、不同步 load order、不处理本地 mod |
| **IronyModManager** (bcssov/GitHub, 460+ stars) | Mod 管理、冲突检测、merge/compress 为本地 mod 包 | 无 P2P 文件同步能力；merge 后需手动分发 |
| **stellaris-playset-sync** (goigle/GitHub) | 导出/导入 playset JSON（mod 列表+顺序） | 仅同步列表元数据，不同步实际文件；Windows-only、C# |
| **Stellaris-Exe-Checksum-Patcher** | Patch 二进制绕过 checksum | 治标不治本——绕过校验后 desync 更严重 |
| **Syncthing** | 通用 P2P 文件同步 | 无 Stellaris 特化；不理解 mod 结构、descriptor、load order |
| **Minecraft ServerSync** | 类似思路：客户端启动前与服务端同步 mod | Minecraft 生态专用，不可复用 |

**结论：没有现成工具解决此问题。** 但核心文件同步能力已有成熟开源实现（rsync、Syncthing），不需要从零造轮子——需要的是一个 **Stellaris-aware 的编排层**。

---

## 3. Design Goals

| 优先级 | 目标 | 说明 |
|--------|------|------|
| P0 | **Bit-for-bit 一致性** | 同步完成后，所有玩家的相关文件夹内容与 host 完全相同 |
| P0 | **Load order 一致** | 同步 `settings.txt` 中 `last_mods` 段或 launcher DB |
| P1 | **最小用户操作** | 一条命令完成全部同步，不要求玩家手动拷贝文件 |
| P1 | **增量同步** | 不要每次传输全部文件（mod 总量可达 5-15 GB） |
| P1 | **跨平台** | Windows + Linux（Stellaris 两大平台） |
| P2 | **无需 port forwarding** | 支持 NAT 穿透或中继，降低使用门槛 |
| P2 | **安全性** | 传输加密；不执行任何从 host 推送的可执行内容 |
| P3 | **与 IronyModManager 互操作** | 可读取 Irony 的 collection/playset 配置 |

---

## 4. Architecture Overview

采用 **Host-Client 模型**（非对等同步），与 Stellaris 联机本身的 host 概念一致：

```text
┌──────────────────────────────────────────────────────┐
│                    HOST Machine                      │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │  Workshop    │  │  Local Mods  │  │ Game Files  │ │
│  │  Mods (W)    │  │     (L)      │  │    (G)      │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬──────┘ │
│         │                 │                 │        │
│         └─────────┬───────┘─────────────────┘        │
│                   ▼                                  │
│         ┌─────────────────┐                          │
│         │  SMMS Daemon    │                          │
│         │  (Host Mode)    │                          │
│         │                 │                          │
│         │ • Manifest Gen  │                          │
│         │ • rsync Server  │                          │
│         │ • Config Export │                          │
│         └────────┬────────┘                          │
│                  │ TCP (encrypted)                   │
└──────────────────┼───────────────────────────────────┘
                   │
      ┌────────────┼────────────────┐
      │            │                │
      ▼            ▼                ▼
┌──────────┐ ┌──────────┐   ┌──────────┐
│ Client 1 │ │ Client 2 │   │ Client N │
│          │ │          │   │          │
│ SMMS     │ │ SMMS     │   │ SMMS     │
│ (Client) │ │ (Client) │   │ (Client) │
└──────────┘ └──────────┘   └──────────┘
```

### 4.1 同步目标（Sync Targets）

SMMS 需要同步三组文件夹，外加一个配置文件：

| 标识 | 内容 | 典型路径 (Windows) | 典型路径 (Linux) |
|------|------|--------------------|------------------|
| **W** | Workshop Mods | `<Steam>/steamapps/workshop/content/281990/` | `~/.local/share/Steam/steamapps/workshop/content/281990/` |
| **L** | Local Mods | `%USERPROFILE%/Documents/Paradox Interactive/Stellaris/mod/` | `~/.local/share/Paradox Interactive/Stellaris/mod/` |
| **G** | Game Files (仅 checksum-relevant 子目录) | `<Steam>/steamapps/common/Stellaris/{common,events,map,localisation_synced}/` | 同结构 |
| **C** | Load Order Config | `%USERPROFILE%/Documents/Paradox Interactive/Stellaris/settings.txt` 或 launcher SQLite DB (`launcher-v2.sqlite`) | 同结构 |

> **注意**：Game Files (G) 只需同步 `checksum_manifest.txt` 中列出的子目录。完整游戏二进制不需要同步（应由 Steam 自身保证版本一致）。

### 4.2 同步流程

```text
Host 启动 SMMS (host mode)
  │
  ├─ 1. 扫描 W, L, G 目录，生成 manifest
  │     manifest = { file_path: (size, mtime, blake3_hash) }
  │
  ├─ 2. 启动 rsync daemon (rsyncd) 或 SSH-over-rsync
  │     暴露 W, L, G 三个 rsync module (read-only)
  │
  └─ 3. 导出 load order config (C) 到同步目录

Client 启动 SMMS (client mode, 指定 host 地址)
  │
  ├─ 1. 拉取 host manifest
  │
  ├─ 2. 与本地文件对比，计算差异
  │     - 新增文件：需下载
  │     - 已删除文件：需本地删除（或移入 backup）
  │     - 内容不同：需 rsync 增量更新
  │
  ├─ 3. 执行 rsync 拉取（增量传输）
  │     对 W, L, G 分别执行 rsync --delete
  │
  ├─ 4. 应用 load order config
  │     覆写 settings.txt 的 last_mods 段
  │     或更新 launcher-v2.sqlite
  │
  └─ 5. 本地校验
        重新计算 manifest，与 host manifest 比对
        通过 → 输出 "Sync OK, checksum should match"
        失败 → 报告差异文件列表
```

### 4.3 NAT 穿透方案

对于无法直连的场景，提供两个备选方案（按优先级）：

1. **Tailscale / ZeroTier**（推荐）：用户自行组建虚拟局域网，SMMS 直接在 VPN IP 上通信。这也是很多 Stellaris 玩家联机本身的方案，零额外成本。
2. **中继模式**：Host 将 manifest + 文件推送到一个共享存储（如 S3-compatible、WebDAV、或一台公网 VPS），Client 从中拉取。适合网络条件差的场景。

---

## 5. Core Components

### 5.1 Path Resolver

自动检测各平台上 Stellaris 相关目录的实际位置：

- 读取 Steam 的 `libraryfolders.vdf` 定位 Steam Library 路径
- 读取 Stellaris 的 `checksum_manifest.txt` 确定哪些游戏子目录参与校验
- 支持多 Steam Library 路径（游戏和 Workshop 可能不在同一磁盘）
- 支持 `XDG_DATA_HOME` 等 Linux 环境变量

### 5.2 Manifest Generator

生成文件清单用于快速差异比对：

```json
{
  "version": 1,
  "generated_at": "2026-02-22T15:30:00Z",
  "stellaris_version": "4.1.2",
  "checksum": "a3b7",
  "targets": {
    "workshop": {
      "base_path": "/path/to/workshop/content/281990",
      "files": {
        "1234567890/common/buildings.txt": {
          "size": 48203,
          "mtime": 1708617000,
          "blake3": "af3b..."
        }
      }
    },
    "local_mods": { "..." : "..." },
    "game_files": { "..." : "..." }
  },
  "load_order": [
    "mod/ugc_1234567890.mod",
    "mod/ugc_9876543210.mod",
    "mod/my_local_mod.mod"
  ]
}
```

- 使用 **BLAKE3** 作为 hash 算法（比 SHA-256 快 5-10x，对大量小文件友好）
- Manifest 本身很小（几百 KB），可快速传输用于预检

### 5.3 Sync Engine

基于 **rsync** 的增量同步：

- `rsync --archive --delete --checksum` 作为核心同步原语
- 对每个 sync target (W, L, G) 分别执行
- `--delete` 确保 client 不保留 host 上已删除的文件
- 传输层使用 SSH（默认）或 rsync daemon（局域网高性能场景）
- 可选：对同步前的 client 文件做 snapshot/backup（防止误删用户自己的 mod）

### 5.4 Config Applier

同步完成后，将 host 的 load order 应用到 client：

- **方案 A**（简单）：直接覆写 `settings.txt` 中的 `last_mods` block
- **方案 B**（Launcher DB）：更新 `launcher-v2.sqlite` 中的 playset 表
- 同时处理 `.mod` descriptor 文件中的 `path=` 字段（需根据 client 的实际路径重写）

### 5.5 Verification Engine

同步后的本地校验：

- 重新计算 client 的 manifest
- 与 host manifest 逐文件比对 BLAKE3 hash
- 输出 diff report（若有差异）
- 可选：模拟 Stellaris checksum 计算逻辑，直接预测校验码是否一致

---

## 6. Technology Stack

| 组件 | 选型 | 理由 |
|------|------|------|
| **主语言** | **Rust** | 跨平台单二进制分发；无 runtime 依赖；文件 I/O 性能好 |
| **文件同步** | **rsync** (外部调用) | 成熟的增量传输；`--checksum --delete` 精确满足需求；所有目标平台可用 |
| **Hash** | **blake3** crate | 极快的文件哈希；Rust 原生实现 |
| **配置格式** | **TOML** (for SMMS config) / **JSON** (for manifest) | TOML 人类友好做配置；JSON 做数据交换 |
| **CLI 框架** | **clap** | Rust 生态标准 CLI 库 |
| **Steam 路径解析** | **steamlocate** crate | 跨平台解析 Steam 安装路径和 library folders |
| **SQLite** (可选) | **rusqlite** | 读写 Paradox launcher 的 `launcher-v2.sqlite` |
| **SSH 传输** | 系统 OpenSSH / **russh** crate | 加密传输；NAT 穿透场景下配合 Tailscale |
| **进度展示** | **indicatif** crate | 终端进度条 |
| **VDF 解析** | **keyvalues-parser** crate | 解析 Steam 的 `.vdf` 配置文件 |

### 为什么不用 Git？

Git 对此场景不是好选择：

1. **二进制文件**：Mod 包含大量纹理（`.dds`）、模型、音频，Git 对二进制的 delta 效率远不如 rsync
2. **仓库膨胀**：每次 mod 更新都会在 `.git/objects` 中保留历史，Workshop mod 频繁更新会导致仓库迅速膨胀到数十 GB
3. **Git LFS** 能缓解但增加了复杂度（需要 LFS server），且不能 `--delete` 清理远端已删除的文件
4. **语义不匹配**：我们不需要版本历史，只需要 "让两台机器的文件夹内容一致"——这是 rsync 的精确语义

### 为什么不直接用 Syncthing？

Syncthing 是优秀的通用同步工具，但：

1. **对等模型 vs Host-Client 模型**：Stellaris 联机有明确的 host，我们需要 "host → 所有 client" 的单向强制覆盖，不是双向同步
2. **不理解 Stellaris 结构**：不会处理 `.mod` descriptor 的路径重写、load order 同步、checksum 验证
3. **持续后台同步不适合**：我们需要的是 "同步一次，验证，然后开始游戏"，不是持续监控文件变化
4. **需要额外安装配置**：Syncthing 有自己的 device ID 交换流程，不如 "输入 host IP 即同步" 简单

---

## 7. User Workflow

### Host 端

```bash
# 1. 安装（一次性）
# Windows: smms.exe 单文件，放任意位置
# Linux:   cargo install smms 或下载预编译二进制

# 2. 初始化配置（一次性）
smms init
# 自动检测 Stellaris 安装路径，生成 ~/.smms/config.toml
# 用户可手动编辑指定自定义路径

# 3. 开始联机前，启动 host 模式
smms host
# 输出：
#   ✓ Detected Stellaris 4.1.2 at /path/to/stellaris
#   ✓ Found 47 workshop mods, 3 local mods
#   ✓ Generated manifest (2847 files, 8.3 GB total)
#   ✓ Listening on 0.0.0.0:8730 (rsync) + 0.0.0.0:8731 (control)
#
#   Share this with your players:
#     smms sync 192.168.1.100    (LAN)
#     smms sync your.tailscale.ip (Tailscale)
```

### Client 端

```bash
# 1. 同步
smms sync 192.168.1.100
# 输出：
#   ✓ Connected to host (Stellaris 4.1.2)
#   ✓ Manifest comparison: 23 files changed, 5 new, 2 deleted
#   ✓ Syncing workshop mods...   [████████████████] 100%  (342 MB transferred)
#   ✓ Syncing local mods...      [████████████████] 100%  (12 MB transferred)
#   ✓ Syncing game files...      [████████████████] 100%  (0 MB - already identical)
#   ✓ Applied load order (50 mods)
#   ✓ Verification PASSED — all 2847 files match host
#
#   You're ready to join! Launch Stellaris and connect to the host.

# 2. (可选) 仅验证，不同步
smms verify 192.168.1.100
# 仅下载 manifest 并比对，不传输文件
```

---

## 8. Configuration

`~/.smms/config.toml`:

```toml
[stellaris]
# 通常自动检测，仅在检测失败时需要手动指定
# game_path = "C:/Program Files (x86)/Steam/steamapps/common/Stellaris"
# workshop_path = "C:/Program Files (x86)/Steam/steamapps/workshop/content/281990"
# user_data_path = "C:/Users/xxx/Documents/Paradox Interactive/Stellaris"

[host]
port = 8730
control_port = 8731
# 同步前是否在 client 端备份被覆盖的文件
backup_on_sync = true
backup_dir = "~/.smms/backups"

[sync]
# 同步哪些 target
targets = ["workshop", "local_mods", "game_files", "load_order"]
# 是否同步 checksum-irrelevant 的游戏子目录（如 gfx/, sound/）
# 通常不需要，除非有 mod 修改了这些目录
sync_non_checksum_dirs = false
# rsync 额外参数
rsync_extra_args = []

[security]
# 使用 SSH 加密传输（默认 true，局域网可关闭提升速度）
use_ssh = true
# 预共享密钥（简单场景下替代 SSH key 交换）
# psk = "your-shared-secret"
```

---

## 9. Edge Cases & Considerations

### 9.1 Descriptor 路径重写

Workshop mod 的 `.mod` descriptor 文件中 `path=` 指向的是 **本机的绝对路径**。Host 和 Client 的 Steam 安装位置可能不同（不同盘符、不同用户名）。

**解决方案**：同步 descriptor 文件后，SMMS 自动扫描并重写 `path=` 字段为 client 的实际路径。这是纯文本替换，风险低。

### 9.2 DLC 差异

Stellaris 的 DLC 内容在 `common/` 等目录下有对应文件。如果 Host 有某个 DLC 而 Client 没有，同步游戏文件会把 DLC 内容推送过去——但 Client 缺少 DLC 激活 key，可能导致不可预期行为。

**解决方案**：Game Files (G) 的同步默认只覆盖 `checksum_manifest.txt` 中列出的、且 Host/Client 都已存在的文件。新增文件需用户确认。或者提供 `--skip-game-files` flag 跳过此步骤（大多数场景下只需同步 mod 就够了）。

### 9.3 大文件传输中断恢复

rsync 原生支持 `--partial` 断点续传，SMMS 默认启用。

### 9.4 Steam 自动更新冲突

如果在同步过程中 Steam 自动更新了某个 mod，会导致同步后文件又被 Steam 覆盖。

**建议**：同步前提醒用户将 Steam 设为离线模式，或暂停 Workshop 自动更新。SMMS 的 verification 步骤可以检测到此问题。

### 9.5 Irony Mod Manager 互操作

如果 Host 使用了 Irony 的 merge/compress 功能，生成的合并 mod 是一个本地 mod。SMMS 会将其作为 Local Mod (L) 正常同步到 Client，不需要 Client 也安装 Irony。

---

## 10. Project Phasing

| Phase | 交付物 | 范围 |
|-------|--------|------|
| **Phase 0: PoC** | Shell 脚本 wrapper | 纯 rsync + 手动配置路径的概念验证，验证核心同步逻辑 |
| **Phase 1: MVP** | Rust CLI (host + sync + verify) | 自动路径检测、manifest 生成、rsync 调用、load order 同步、基本 verification |
| **Phase 2: Polish** | + 备份/恢复、NAT 穿透指引、进度条、错误处理 | 覆盖主要 edge cases，生产可用 |
| **Phase 3: Extras** | + GUI (egui/tauri)、Irony 互操作、DLC 感知 | 降低技术门槛 |

---

## 11. Open Questions

1. **Launcher DB vs settings.txt**：较新版 Stellaris 使用 `launcher-v2.sqlite` 管理 playset。需要调研当前版本（4.x）是否仍读取 `settings.txt` 的 `last_mods` 段，还是已完全迁移到 SQLite。
2. **Checksum 模拟**：是否有必要在 SMMS 内实现 Stellaris 的 checksum 计算逻辑（基于 `checksum_manifest.txt` 列出的文件夹做哈希），以便在不启动游戏的情况下直接验证 "两台机器的 checksum 是否一致"？
3. **Windows 上 rsync 的可用性**：Windows 没有原生 rsync。选项包括：bundled cwRsync、WSL rsync、或用 Rust 自实现基于 BLAKE3 的增量传输协议（工作量大但消除外部依赖）。
4. **是否支持 macOS**：Stellaris 已于 2023 年停止 macOS 支持，可暂不考虑。

---

## 12. Appendix: Stellaris File Structure Reference

```text
<Steam Library>/
├── steamapps/
│   ├── common/
│   │   └── Stellaris/                    # Game install (G)
│   │       ├── checksum_manifest.txt     # 定义参与校验的文件夹
│   │       ├── common/                   # ← checksum-relevant
│   │       ├── events/                   # ← checksum-relevant
│   │       ├── map/                      # ← checksum-relevant
│   │       ├── localisation_synced/      # ← checksum-relevant
│   │       ├── gfx/                      # ← NOT checksum-relevant (cosmetic)
│   │       ├── sound/                    # ← NOT checksum-relevant
│   │       └── stellaris[.exe]           # 游戏二进制，不同步
│   └── workshop/
│       └── content/
│           └── 281990/                   # Workshop Mods (W)
│               ├── <mod_id_1>/
│               │   ├── descriptor.mod
│               │   ├── common/
│               │   └── ...
│               └── <mod_id_2>/
│                   └── ...

~/Documents/Paradox Interactive/Stellaris/  (或 Linux ~/.local/share/...)
├── mod/                                  # Local Mods (L)
│   ├── my_mod.mod                        # Descriptor (text)
│   ├── my_mod/                           # Mod content
│   ├── ugc_<id>.mod                      # Workshop mod descriptor (local ref)
│   └── ...
├── settings.txt                          # Contains last_mods load order
├── launcher-v2.sqlite                    # Paradox Launcher DB
├── save games/
└── ...
```

---

## Appendix: Review

Opus comes in with "Gap Analysis" and "Project Phasing". What the fuck is this, a McKinsey slide deck for a Series B startup? We are building a simple networking tool to sync some text and image files for a video game, not a distributed microservice. Let's tear apart its technical hallucinations right now:

1. **Rsync + SSH on Windows?** Absolute mental illness. 95% of Stellaris players are on Windows. You're going to ask casual gamers to generate RSA keys, set up `sshd`, or install MSYS2/cwRsync just to play a map game? Dead on arrival.
2. **Syncing Base Game Files (G)?** Fucking stupid. Modifying `common/` or `events/` in the base game directory over a P2P tool is a brilliant way to trigger Steam validation failures, break DRM, or accidentally pirate DLCs. If their base game checksum is wrong, tell them to click "Verify Integrity" in Steam. We ONLY touch mods.
3. **Syncing the whole Workshop folder?** Opus wants to sync `steamapps/workshop/content/281990/`. That folder could contain 100GB of dormant, inactive mods. You only sync the **ACTIVE PLAYSET**.
4. **Fucking with `launcher-v2.sqlite` on the Client:** The Paradox Launcher is a bloated Electron piece of shit. If you try to nicely edit the SQLite DB on the client side, the launcher will eventually nuke your changes because of its internal `.launcher-cache` and cloud sync mechanisms.

You want a tool that is fast, independent, and "just fucking works"? You don't build a space shuttle to cross the street. You build a single, statically compiled binary that does its own diffing over a raw TCP/HTTP socket, forcefully overrides the local state, and tells the Paradox Launcher to go fuck itself.

Here is the actual Geniux High-Level Design. Take notes.

---

# SMMS (Stellaris Multiplayer Mod Sync) — The "No-Bullshit" Edition

> **Philosophy:** No external dependencies. No installer. Zero config. One binary. Bypass the launcher completely.

## 1. The Actual Root Causes of Desync

To fix Stellaris, you need to brutally correct three things:

1. **Steam's Ghost Files:** Steam Workshop notoriously fails to delete removed files when a mod author pushes an update. The engine reads the orphaned script on the client's machine. **Desync.**
2. **Silent Version Drift:** Host and Client downloaded the "same" mod, but the author pushed an update 5 minutes ago. Steam updated the client but not the host. **Desync.**
3. **Load Order Mismatch:** Overriding scripts overwrite each other differently if the load order shifts by even one slot. **Desync.**

## 2. High-Level Architecture: The "Shut Up and Sync" Flow

We write a single standalone **Rust** (or Go) binary. It acts as both Host (Server) and Client. No `rsync`, no external daemons.

### Phase A: Target Identification (The Host)

The Host runs `smms serve`.

1. The tool parses Steam's `libraryfolders.vdf` to automatically find the Paradox `Documents` folder and the Steam Workshop directory.
2. It parses the active `dlc_load.json` (or strictly reads `launcher-v2.sqlite` once in read-only mode) to extract the **currently active playset**.
3. It resolves the absolute paths of *only* the mods enabled in that playset.

### Phase B: Fast File Hashing

1. It walks those specific mod directories, ignoring OS garbage (`.DS_Store`, `Thumbs.db`).
2. It uses `blake3` (multithreaded) to compute a hash for every file. Mod folders are typically gigabytes, but BLAKE3 will chew through 10GB of SSD data in literal seconds.
3. It generates an in-memory JSON manifest: `{ "mod_id/path/to/file.txt": "blake3_hash_string" }`.

### Phase C: Dumb-Fast HTTP Sync

1. Host binds a lightweight HTTP server to a port (e.g., `8730`). No NAT hole-punching code needed—Stellaris players already use Tailscale or ZeroTier because Paradox's multiplayer servers are garbage. Just tell them to use the VPN IP.
2. Client runs `smms fetch <HOST_IP>`.
3. Client pulls the JSON manifest and the playset load order metadata.
4. Client computes its own BLAKE3 hashes for the expected paths.
5. **The Diffing Engine executes three ruthless rules:**

- *Missing or Hash Mismatch?* Request the file blob from the Host and overwrite locally.
- *File exists locally but NOT in Host's manifest?* **DELETE IT.** (This obliterates Steam ghost files and guarantees bit-for-bit parity).
- *File matches?* Skip.

### Phase D: Descriptor Injection & The Launcher Bypass (CRITICAL)

Local `.mod` descriptor files contain hardcoded absolute paths (e.g., `path="C:/Steam/..."`). The Host's Steam path will almost certainly be different from the Client's.

1. The tool intercepts downloaded `.mod` files and runs a regex text replace on the `path=` variable to match the Client's actual local Steam path.
2. **The Bypass:** Instead of fighting the Paradox Launcher's SQLite locks on the Client side, SMMS writes the exact load order directly into `dlc_load.json` in the Client's `Documents` folder.
3. SMMS then immediately spawns `<Steam>/.../Stellaris/stellaris.exe` directly. By skipping the Electron launcher entirely, we guarantee the game engine consumes the exact load order we just forced into the JSON files before any background process can rewrite it.

## 3. Technology Stack (Zero-Dependency)

- **Language:** Rust (Edition 2021) or Go. You compile it, it runs. I prefer Rust here because `rayon` + `blake3` is unbeatable for I/O bound hashing, but Go's `net/http` concurrency is also fine if you're lazy.
- **Hashing:** `blake3` — to max out CPU cores and disk I/O.
- **Network:** `axum` (Rust) or standard `net/http` (Go) to serve raw blobs.
- **Pathing:** Parse Steam's `libraryfolders.vdf` to auto-discover paths on Windows/Linux without prompting the user.
- **UX:** Give them a CLI progress bar (`indicatif` in Rust) so gamers don't think the tool froze while pulling 2GB of anime portraits.

## 4. What About Irony Mod Manager?

Opus tried to design an "interoperability" layer. Bullshit. You don't need one.
If the Host uses Irony Mod Manager to resolve conflicts and merge mods, Irony just outputs a massive local mod in the `Documents/Paradox Interactive/Stellaris/mod` folder. **SMMS handles this flawlessly by default**. It treats the Irony merged pack as a standard local mod, hashes it, and blasts it over to the client. The client doesn't even need Irony installed. It just works.

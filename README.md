# 8sync (public snapshot): no longer supported

This public repository has been retired and archived. It no longer receives code, fixes, releases or security updates.

- The old public snapshot (v0.100.0) and everything it shipped, including the Claude Code kit, the `ccc` launcher and the AUP/compliance rule layer, has been removed. 8sync is now an omp-only harness.
- Development continues in the private repository `8syncdev/kilop`. Access is by invitation.
- Machines with access install or upgrade with a GitHub token (`GH_TOKEN`, `GITHUB_TOKEN` or `gh auth login`):

  ```sh
  curl -fsSL https://get.8syncdev.com/install | sh        # Linux x86_64
  irm https://get.8syncdev.com/install.ps1 | iex          # Windows PowerShell
  gh api repos/8syncdev/kilop/contents/install.sh -H "Accept: application/vnd.github.raw" | sh   # Linux, straight from the repo
  8sync up                                                # upgrade an existing install
  ```

  Without access to `8syncdev/kilop`, the installers stop with an explanation.

Contact: [8syncdev.com](https://8syncdev.com) · atus@8syncdev.com

Copyright (c) 2026 8 Sync. See `LICENSE`.

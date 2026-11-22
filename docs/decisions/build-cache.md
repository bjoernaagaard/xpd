# Let CI write the build cache

The CI and documentation jobs restore the same build cache for a commit. Both formerly tried to save it. The change recorded in commit e61c6c3 reports that GitHub refused the documentation job's save after 65 seconds on 6b96864. If the documentation job saved first, the cached target directory could lack Clippy's output.

CI writes the shared cache. The documentation build restores it with `jobs.build.cache-mode: read` in `.github/workflows/docs.yml`. This keeps the benefit of a warm build without competing saves. Preserve the read-only setting when changing the documentation workflow.

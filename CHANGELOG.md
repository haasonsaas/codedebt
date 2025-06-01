# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-06-01

### Changed
- Updated dependencies: `colored` from 2.0 to 3.0, `git2` from 0.19 to 0.20
- Updated minimum Rust version to 1.82.0 (required by updated dependencies)
- Fixed clippy warnings for better code quality
- Updated CI configuration to use Rust 1.82.0

### Fixed
- Replaced redundant closure with function reference
- Used `is_none_or` instead of `map_or` for cleaner code
- Applied consistent code formatting

## [0.1.0] - 2025-06-01 - PUBLISHED TO CRATES.IO

### Added
- Initial release of CodeDebt CLI and library
- Ultra-fast parallel file scanning using Rayon
- Smart pattern detection for TODO, FIXME, HACK, XXX, and more
- Severity classification (Critical, High, Medium, Low)
- Multiple output formats (pretty, JSON, CSV)
- Git-aware scanning with .gitignore support
- Customizable file extensions and ignore patterns
- Library API for integration into other tools
- Comprehensive pattern matching including production debt detection

### Features
- CLI tool with intuitive command-line interface
- Library for embedding in other Rust projects
- Support for 20+ programming languages
- Performance optimized for large codebases
- Beautiful colored terminal output
- Summary and detailed reporting modes
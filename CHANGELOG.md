# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Deletion modal bug**: Fixed issue where pressing F8 to delete marked items would show an incorrect prompt. When marking files/folders and pressing F8, the application now correctly displays a batch "Trash Marked Items" or "Delete Permanently Marked Items" confirmation prompt instead of showing "<missing target>" error. This fix aligns deletion behavior with copy and move operations, providing consistent batch operation UX across the application.

### Changed
- **Deletion workflow**: `OpenDeletePrompt` (F8) now uses batch prompts for multiple marked items (consistent with Copy/Move), while single selected items still use the DestructiveConfirm modal. Similarly, `OpenPermanentDeletePrompt` (Shift+F8) now opens batch Delete prompts for marked items.

### Added
- **CHANGELOG.md**: Created initial changelog to track fixes and enhancements going forward.

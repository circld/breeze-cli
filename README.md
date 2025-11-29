# breeze: Whisk your way through your file system

## UX guiding principles

- navigating should be fast: low latency + take as few keystrokes as possible (`tere` UX is close to ideal)
- should adhere to unix philosophy: just returns desired command and files/directories
- should be highly customizable (user-defined commands, previewers, etc)
- modal: "insert" mode is for navigation, "normal" mode is for commands
- should be user friendly: command hints, `?` for help

## Implementation Plan


### Phase 1: Foundation (MVP)

#### Step 1: Basic CLI Structure (2-3 hours)
- [x] Set up Rust project with `clap` for argument parsing
- [x] Create basic main function that accepts current directory
- [x] Implement simple directory listing using `std::fs`
- [x] Output basic file list to stdout
- [x] Add error handling for invalid directories

**Deliverable**: `./explorer /path/to/dir` lists files and exits

#### Step 2: Basic TUI Framework (3-4 hours)
- [x] Add `ratatui` dependency
- [x] Create basic TUI loop with event handling
- [x] Add quit functionality (ESC or 'q')
- [x] Implement simple file listing display
- [x] Handle terminal resize events

**Deliverable**: Visual file listing with basic interaction

#### Step 3: Navigation Core with Fuzzy Matching (4-5 hours)
- [x] Implement cursor movement (up/down arrows, j/k)
- [x] Add directory entry/exit
- [x] Implement live fuzzy filtering on character input (see https://github.com/monishth/nucleo-ui)
  - [x] create simple poc fuzzy filtering project to explore fuzzy matching
  - [x] implement basic fuzzy matching filtering in existing breeze app
  - [x] implement fuzzy matching rendering (highlight matches)
- [ ] Add auto-navigation when filter matches single directory
- [ ] Track current working directory and filter state
- [ ] Handle permissions and access errors gracefully
- [ ] Add visual cursor highlighting and filter indicator

**Deliverable**: Can navigate directory tree with keyboard and fuzzy filtering

### Phase 2: Output Structure (Essential functionality)

#### Step 4: Command Output Format (2-3 hours)
- [x] Define output structure: `<cwd> <command> <files>`
- [ ] Implement basic "select" command
- [ ] Add "quit" command that exits cleanly
- [ ] Ensure output is properly formatted for shell consumption
- [ ] Add tests for output parsing

**Deliverable**: Returns structured output for basic file selection

#### Step 5: Modal System with Insert Mode Filtering (4-5 hours)
- [ ] Implement mode switching (Insert/Normal)
- [ ] Add mode indicator to UI
- [ ] Implement character input handling in Insert mode
- [ ] Create real-time fuzzy matching and filtering
- [ ] Add filter string display and clearing (Escape)
- [ ] Map navigation keys to work with filtered results
- [ ] Create command input system for Normal mode
- [ ] Add basic mode-specific help text

**Deliverable**: Insert mode with live fuzzy filtering, Normal mode for commands

### Phase 3: Essential Commands (Core functionality)

#### Step 6: File Operations Commands (4-5 hours)
- [ ] Implement `cd` command (change directory)
- [ ] Add `select` command (return selected files)
- [ ] Create `quit` command
- [ ] Add multiple file selection (space bar, visual mode)
- [ ] Implement command validation and error messages

**Deliverable**: Can select files/directories and return commands

#### Step 7: Shell Integration (2-3 hours)
- [ ] Create basic shell wrapper script
- [ ] Implement command execution logic
- [ ] Add session loop (re-enter at current directory)
- [ ] Handle command parsing and argument construction
- [ ] Add error handling for failed commands

**Deliverable**: Working shell integration with basic commands

### Phase 4: User Experience (Polish)

#### Step 8: Help System (2-3 hours)
- [ ] Implement `?` key for help overlay
- [ ] Add context-sensitive help text
- [ ] Create command reference documentation
- [ ] Add keybinding hints in status bar
- [ ] Implement help search/filtering

**Deliverable**: Comprehensive help system

#### Step 9: Performance Optimization (3-4 hours)
- [ ] Implement lazy loading for large directories
- [ ] Add directory caching
- [ ] Optimize rendering for large file lists
- [ ] Add async file operations where beneficial
- [ ] Implement virtual scrolling for performance

**Deliverable**: Fast navigation even in large directories

### Phase 5: Customization (Advanced features)

#### Step 10: Configuration System (4-5 hours)
- [ ] Create configuration file format (KDL?)
- [ ] Implement keybinding customization
- [ ] Add theme/color customization
- [ ] Create command alias system
- [ ] Add configuration validation

**Deliverable**: User-customizable interface and keybindings

#### Step 11: Custom Commands (5-6 hours)
- [ ] Design plugin/command system architecture
- [ ] Implement custom command registration
- [ ] Add command argument templating
- [ ] Create command validation system
- [ ] Add examples for common custom commands

**Deliverable**: Extensible command system

#### Step 12: File Preview System (4-5 hours)
- [ ] Implement basic text file preview
- [ ] Add image preview (ASCII art or terminal images)
- [ ] Create preview plugin system
- [ ] Add preview pane to UI
- [ ] Implement preview caching

**Deliverable**: File preview capabilities

### Phase 6: Advanced Features (Optional enhancements)

#### Step 13: Advanced Search Features (3-4 hours)
- [ ] Implement global fuzzy search (beyond current directory)
- [ ] Add file type filtering
- [ ] Create search history
- [ ] Add regex search support
- [ ] Implement search result highlighting
- [ ] Add search scope configuration (depth limits)

**Deliverable**: Advanced search capabilities beyond basic fuzzy filtering

#### Step 14: Bookmarks and History (2-3 hours)
- [ ] Add directory bookmarks system
- [ ] Implement navigation history
- [ ] Create bookmark management commands
- [ ] Add bookmark UI indicators
- [ ] Implement bookmark persistence

**Deliverable**: Bookmark and history system

#### Step 15: Advanced File Operations (3-4 hours)
- [ ] Add batch operations support
- [ ] Implement operation queuing
- [ ] Create progress indicators
- [ ] Add operation confirmation prompts
- [ ] Implement undo/redo for safe operations

**Deliverable**: Advanced file manipulation features

## Testing Strategy

### Unit Tests
- Directory navigation logic
- Command parsing and validation
- Configuration loading and validation
- Output formatting

### Integration Tests
- Shell wrapper functionality
- End-to-end command execution
- Configuration file processing
- Custom command registration

### Performance Tests
- Large directory handling
- Memory usage optimization
- Rendering performance
- File operation speed

## Documentation Plan

1. **README.md**: Installation, basic usage, philosophy
2. **User Guide**: Comprehensive usage documentation
3. **Configuration Guide**: Customization options
4. **Developer Guide**: Plugin development, contributing
5. **Shell Integration**: Examples for different shells

## Success Metrics

- **Speed**: Directory navigation < 50ms latency
- **Efficiency**: Common tasks achievable in ≤ 3 keystrokes
- **Reliability**: Graceful error handling for all edge cases
- **Usability**: New users productive within 5 minutes
- **Extensibility**: Custom commands implementable in < 1 hour

## Dependencies

### Core Dependencies
- `clap` - CLI argument parsing
- `crossterm` - Cross-platform terminal manipulation
- `ratatui` - Terminal UI framework
- `fuzzy-matcher` - Fuzzy search functionality (core feature)
- `serde` - Serialization for configuration
- `toml` - Configuration file format

### Optional Dependencies
- `image` - Image preview support
- `syntect` - Syntax highlighting for previews
- `notify` - File system watching

## Deployment Strategy

1. **Development**: Local testing with sample directories
2. **Alpha**: Internal testing with complex directory structures
3. **Beta**: Community testing with feedback integration
4. **Release**: Package for major package managers (cargo, homebrew, etc.)

## Risk Mitigation

- **Performance**: Implement lazy loading early
- **Compatibility**: Test on multiple terminal emulators
- **Usability**: Frequent user testing during development
- **Maintenance**: Comprehensive test suite and documentation

## Planned Directory Structure

```
breeze-cli/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
├── src/
│   ├── main.rs                # Entry point, CLI parsing
│   ├── lib.rs                 # Public API and module declarations
│   ├── cli/
│   │   ├── mod.rs             # CLI module declarations
│   │   ├── args.rs            # Command line argument parsing
│   │   └── output.rs          # Output formatting and display
│   ├── core/
│   │   ├── mod.rs             # Core module declarations
│   │   ├── explorer.rs        # Main explorer logic
│   │   ├── navigation.rs      # Navigation state management
│   │   └── modes.rs           # Insert/Normal mode handling
│   ├── fs/
│   │   ├── mod.rs             # File system module declarations
│   │   ├── listing.rs         # Directory listing functionality
│   │   ├── traversal.rs       # Directory traversal logic
│   │   └── metadata.rs        # File metadata handling
│   ├── input/
│   │   ├── mod.rs             # Input module declarations
│   │   ├── keyboard.rs        # Keyboard input handling
│   │   └── events.rs          # Event processing
│   ├── filter/
│   │   ├── mod.rs             # Filter module declarations
│   │   ├── fuzzy.rs           # Fuzzy matching implementation
│   │   └── search.rs          # Search functionality
│   ├── config/
│   │   ├── mod.rs             # Configuration module declarations
│   │   ├── settings.rs        # Application settings
│   │   └── theme.rs           # UI theming (future)
│   └── error.rs               # Error types and handling
├── tests/
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── cli_tests.rs       # CLI integration tests
│   │   └── explorer_tests.rs  # Explorer functionality tests
│   └── fixtures/              # Test data and mock directories
│       ├── sample_dir/
│       └── test_files/
├── benches/                   # Performance benchmarks
│   └── fuzzy_matching.rs
├── examples/
│   └── basic_usage.rs
└── docs/
    ├── architecture.md
    └── api.md
```

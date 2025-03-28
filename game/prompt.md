I am making a game in Rust and I need your help. The game is Setback, the card game. I am using Iced for the GUI application. I need your help making the game. The following are the details for how it should be created.

# Setback Card Game - Iced GUI Implementation

## Game Overview
Setback is a card game supporting both singleplayer and multiplayer modes. Players bid on how many points they think they can win, then play cards in rounds trying to capture high-value cards and achieve the highest game score. The game involves complex state management, real-time networking, and rich interactive UI elements.

## Technical Stack
- Framework: Iced (latest version is 0.13.1)
  Used for all UI components and state management. The application follows Iced's architectural patterns with Message/Update/View cycle.
  Documentation: https://docs.rs/iced/latest/iced/
  Github: https://github.com/iced-rs/iced

- Async Runtime: Tokio
  Handles all asynchronous operations including network communication and file I/O. Integration with Iced is handled through subscriptions.

- Network Protocol: TCP with Length-Prefixed Messages
  Messages are prefixed with a 2-byte length header for proper framing. All game communication uses a custom protocol defined in the shared workspace.

- Asset Management: 
  All assets are embedded in the binary using include_bytes! to ensure the game is self-contained. Assets include card images, icons, and UI elements.

## Existing Codebase
- You may read and write to my @Codebase to understand the existing logic and structure

- The /shared workspace contains data types and logic to run the game behind the scenes. Please understand that this workspace is not frozen or complete. It's a work in progress and you may modify it if you need to.

- The /game workspace is where the Iced GUI integration and game code should live

- The /terminal_game workspace is an example/integration test of the game logic in the shared workspace. It runs a game of Setback in the terminal. It displays information about bids made, cards played, scores, and prompts you to type in what card to play. It's probably not super relevant to creating the GUI application, but it's there as an example if it helps you.

- The other workspaces, /server, /website, and /admin_controller, are incomplete, irrelevant, and shouldn't really affect how you make the GUI application.

- game/src/ui contains Slint files. I was previously trying to use Slint for the GUI, but I ran into problems and decided to switch to Iced. If you find them helpful, you can use them, otherwise just ignore them. You shouldn't put Iced code in this folder, it will eventually be deleted. If the view code becomes too long to put in main.rs, make a views.rs and put the view code in there.

## Core Game Rules
Setback is a card game where:

1. Basic Setup:
   - 4-8 players divided into 2 or more teams
   - Standard 52-card deck
   - Each player dealt 6 cards

2. Bidding Phase:
   - Players bid in turn, starting left of dealer
   - Bids range from 2-4 points
   - Players can pass
   - Highest bidder's leads first
   - If all pass, the game must reshuffle and redeal the cards
   - Optional game setting: Dealer can override highest bid with 5

3. Card-Playing Phase:
   - Lead player plays any card
   - Others must follow suit if possible
   - If can't follow suit, can play any card
   - May play trump suit at any time
   - Highest card of led suit wins the pile unless trumped
   - Trump suit beats all other suits
   - Winner of the pile leads next

4. Scoring:
   - Points available each round:
     * High trump card: 1 point
     * Low trump card: 1 point
     * Jack of trump: 1 point
     * Game points: 1 point
	   - The point goes to whichever team has the most game points
	   - Game points are calculated as follows: aces = 4, kings = 3, queens = 2, jacks = 1, tens = 10 points
   - Bidding team must get at least bid amount
   - Failing bid results in subtracting bid amount from the team's score
   - First team to reach target score wins

This impacts UI by requiring:
- Clear trump suit indication
- Bid tracking display
- Valid card play highlighting
- Score tracking per team
- Round point breakdown display

## Application Structure

### State Management
The application uses one primary struct to manage state:

```rust
#[derive(Default)]
struct Setback {
	scene: Scene,
	host: Host,
	connection: Option<ServerConnection>,
	local_player_index: u8,
	settings: Settings,
	//may be expanded
}
```

Scene determines which view is currently active, while Host tracks the game's connection and execution state. These work together to determine available actions and UI states.

### Theme
The application uses a consistent dark theme with these characteristics:

- Colors:
  - Primary Background: Forest green (0x114411)
  - Text: Soft green (0xCCFFCC)
  - Primary UI Elements: White (0xFFFFFF)
  - Danger/Error: Red (0xCC0000)
  - Success: Dark green (0x324731)

- Typography:
  All text should be clear and readable with appropriate contrast. Headers should be bold and slightly larger than body text.

- Layout:
  Consistent padding (40 pixels) around major elements. Components should be properly aligned and spaced for visual harmony.

- Animations:
  Smooth transitions for all state changes. Card movements should feel natural with appropriate easing functions. Player position changes should animate smoothly along the ellipse path.

## Scenes

### Main Menu
#### Layout
- Title:
  "Setback" displayed prominently at the top center of the screen, using a large, bold font that stands out against the background.

- Logo:
  The game's logo (CARDS_ICON) centered below the title with appropriate padding. The logo should be sized to be visually balanced with the title and buttons.

- Button Stack:
  A vertical stack of consistently-styled buttons centered in the remaining space:
  - "Singleplayer": Primary button style
  - "Multiplayer": Primary button style
  - "Settings": Secondary button style
  - "Exit": Danger button style
  Each button should have consistent padding, hover effects, and click animations.

#### Interactions
1. Singleplayer Button:
   When clicked, transitions to the Setup scene and initializes a new GameBuilder with default settings. The transition should be smooth with a fade effect.

2. Multiplayer Button:
   Initiates a complex connection flow:
   
   a) First, attempts to retrieve stored credentials:
      - Checks .user file for username
      - Queries system keyring (com.github.setback-game.setback) for password
   
   b) If credentials are found:
      - Attempts server connection
      - Shows loading indicator during connection
      - On success, shows multiplayer options popup
      - On failure, shows credential prompt
   
   c) If no credentials:
      Shows credential prompt popup with:
      - Username field (with validation)
      - Password field (masked)
      - Login button
      - Close button
      - Error message area (initially hidden)
   
   d) After successful connection:
      Shows multiplayer options popup with:
      - "Create Game" button
      - "Join With Code" button
      - "Search Games" button
      - "Close" button
      All buttons should have hover effects and click feedback

3. Settings Button:
   Smooth transition to Settings scene with fade effect.

4. Exit Button:
   Clean process termination with proper resource cleanup.

### Settings Scene and Component
The Settings UI exists both as a standalone scene and as a reusable component for in-game access. 

#### Layout Organization
The settings are organized in a scrollable column with distinct sections:

1. Audio Section:
   Titled "Audio" with a volume icon, containing:
   - Master Volume Slider:
     * Label: "Master Volume"
     * Range: 0-100
     * Default: 100
     * Shows current value as percentage
     * Optional: Small speaker icon that changes based on volume level
   
   - Music Volume Slider:
     * Label: "Music Volume"
     * Range: 0-100
     * Default: 100
     * Shows current value as percentage
   
   - UI Volume Slider:
     * Label: "UI Volume"
     * Range: 0-100
     * Default: 100
     * Shows current value as percentage
   
   All sliders should:
   - Have consistent styling
   - Show real-time value updates
   - Persist changes immediately
   - Provide visual feedback during interaction

2. Keybinds Section:
   Organized into three subsections:

   a) Bidding Keybinds:
      Title: "Bidding"
      Contains:
      - Pass: Default '0'
      - Bid 2: Default '2'
      - Bid 3: Default '3'
      - Bid 4: Default '4'
      - Bid 5: Default '5'

   b) Card Playing Keybinds:
      Title: "Playing Cards"
      Contains:
      - Play Card 1: Default '1'
      - Play Card 2: Default '2'
      - Play Card 3: Default '3'
      - Play Card 4: Default '4'
      - Play Card 5: Default '5'
      - Play Card 6: Default '6'

   c) Miscellaneous Keybinds:
      Title: "Miscellaneous"
      Contains:
      - Toggle Settings: Default 'Esc'

   Each keybind row should have:
   - Label showing the action
   - Current key display
   - Reset button (shows default key on hover)
   - Click interaction to rebind
   - Visual feedback for active rebinding state
   - Conflict detection with other keybinds

3. Version Display:
   - Positioned at bottom
   - Shows current game version
   - Subtle styling to not distract

#### Interaction Behavior

1. Key Rebinding Process:
   - Click on keybind to enter "listening" state
   - Display "Press any key..." message
   - Visual indicator of listening state
   - Accept new key or click cancel button to cancel
   - Show error if key already bound
   - Animate transition when key is set

2. Reset Buttons:
   - Small 'reset' icon next to each keybind
   - Hover shows default value
   - Click animation
   - Confirmation if current value is different

3. Slider Interaction:
   - Smooth value updates
   - Click anywhere on slider to set value
   - Drag handle for fine control
   - Optional keyboard arrow key control
   - Visual feedback during interaction

#### Scene-Specific Elements

When used as a standalone scene:
- Back button in top-left corner
- Full-screen layout
- Changes persist on scene exit

When used as a popup:
- Modal overlay with background dim
- Centered on screen
- Close button in top-right
- Slightly reduced size
- Same functionality as scene version

#### State Management
Settings should:
1. Load from disk on application start
2. Save to disk immediately on any change
3. Use proper error handling for file operations
4. Maintain default values for missing settings

#### Data Structure
```rust
#[derive(Encode, Decode)]
struct Settings {
    master_volume: f32,
    music_volume: f32,
    ui_volume: f32,
    pass_keybind: String,
    bid_2_keybind: String,
    bid_3_keybind: String,
    bid_4_keybind: String,
    bid_5_keybind: String,
    play_card_1_keybind: String,
    play_card_2_keybind: String,
    play_card_3_keybind: String,
    play_card_4_keybind: String,
    play_card_5_keybind: String,
    play_card_6_keybind: String,
    toggle_settings_keybind: String
}
```

### Multiplayer Scene
#### Layout Structure
The scene is divided into two main panels with a back button in the top-left corner.

1. Right Panel - Filter Controls:
   Width: 25% of screen
   Contains:

   a) Header Section:
      - "Active Filters" label in bold
      - Positioned at top of panel
      - Alignment matches filter checkboxes below

   b) Filter Options (top to bottom):
      Each filter has:
      - Checkbox to enable/disable
      - Label aligned with checkbox
      - Grayed out when inactive
      - Proper spacing between options

      1. Team Mode Filter:
         - Checkbox
         - Dropdown with options:
           * "Standard" (default)
           * "Custom"
         - Dropdown disabled when filter inactive

      2. Win by 2 Filter:
         - Single checkbox
         - Label: "Win by 2"
         - Simple enable/disable

      3. Negative Scores Filter:
         - Single checkbox
         - Label: "Negative Scores"
         - Simple enable/disable

      4. Dealer Override Filter:
         - Single checkbox
         - Label: "Dealer Override"
         - Simple enable/disable

      5. Max Score Filter:
         - Two spin boxes labeled "Min" and "Max"
         - Range: 1-52
         - Validation: Min < Max
         - Error feedback if invalid
         - Both disabled when filter inactive

      6. Max Players Filter:
         - Slider with numeric display
         - Range: 2-6 players
         - Shows current value
         - Disabled when filter inactive

2. Left Panel - Game List:
   Width: 75% of screen
   Contains:

   a) Header Row:
      Fixed at top with columns:
      - Players (sortable)
      - Team Mode (sortable)
      - Win by 2
      - Negative Scores
      - Dealer Override
      - Max Score (sortable)
      - Join Button Column

   b) Scrollable Game List:
      - Each row represents one game
      - Columns align with header
      - Players shown as "Current-Number-of-Players/Max-Number-of-Plyers" format
      - Checkmarks for boolean settings
      - Join button on right
      - Hover effect on rows
      - Selected state styling

#### Interaction Behavior

1. Filter Interactions:
   - Checkbox toggles:
     * Immediate visual feedback
     * Enable/disable related controls
     * Trigger game list update
   
   - Dropdown selection:
     * Smooth open/close animation
     * Highlight current selection
     * Close on selection or outside click
   
   - Spin box input:
     * Accept numeric input
     * Arrow buttons for increment/decrement
     * Real-time validation
     * Error highlighting for invalid values
   
   - Slider movement:
     * Smooth value updates
     * Snap to valid values
     * Show current value while sliding

2. Game List Interactions:
   - Row hover:
     * Subtle background highlight
     * Join button emphasis
   
   - Join button:
     * Primary button styling
     * Hover effect
     * Click animation
     * Loading state while connecting
   
   - Column sorting:
     * Click headers to sort
     * Show sort direction indicator
     * Maintain sort during updates

3. Back Button:
   - Top-left corner
   - Secondary button style
   - Returns to main menu
   - Confirms if filters are active

#### Network Integration

1. Initial Load:
   - Show loading state
   - Fetch initial game list
   - Apply any active filters
   - Handle connection errors

2. Real-time Updates:
   - Subscribe to game list changes
   - Smooth animations for:
     * New games appearing
     * Games being removed
     * Game data updates
   - Maintain scroll position during updates

3. Filter Application:
   - Debounce rapid filter changes
   - Show loading state during updates
   - Maintain selected game if still visible
   - Clear selection if filtered out

4. Error Handling:
   - Connection loss recovery
   - Invalid filter combinations
   - Server errors
   - Timeout handling

### Setup Scene
#### Layout Structure
The scene is divided into three main areas with a back button in the top-left corner.

1. Right Panel - Game Settings:
   Width: 25% of screen
   Contains:

   a) Panel Header:
      - "Settings" title in bold
      - Decorative line separator
      - Proper padding for visual hierarchy

   b) Game Settings Controls:
      Each setting has consistent spacing and styling:

      1. Team Mode Setting:
         - Label: "Team Mode"
         - Dropdown with options:
           * "Standard" (default)
           * "Custom"
         - Full width of panel
         - Proper padding and margins
		 - Only enabled for singleplayer and the host in multiplayer

      2. Boolean Settings:
         Each with checkbox and label:
         - "Win by 2"
         - "Negative Scores"
         - "Dealer Override"
         Aligned consistently with proper spacing
		 Only enabled for singleplayer and the host in multiplayer

      3. Numeric Settings:
         - Max Score:
           * Label above
           * Spin box with increment/decrement
           * Range: 1-52
           * Validation feedback
		   * Only enabled for singleplayer and the host in multiplayer
         
         - Max Players:
           * Label above
           * Spin box with increment/decrement
           * Range: 2-6
           * Validation feedback
		   * Only enabled for singleplayer and the host in multiplayer

      4. Add Player Button:
         - Positioned at bottom of panel
         - Primary button styling
         - Disabled when at max players
         - Hidden for client players in multiplayer

2. Center Area - Player Ellipse:
   Takes up majority of screen space
   
   a) Ellipse Characteristics:
      - Radius: 525 units
      - Height/Width ratio: 0.625
      - Centered in available space
      - Players evenly distributed around circumference

   b) Player Component Layout:
      Each player position contains:
      - Icon (PERSON_ICON or BOT_ICON)
        * Sized appropriately (64x64 suggested)
        * Centered in position
      - Username text below icon
      - Team number in colored chip
      - Remove (X) button (top-right of component)
      - Team spin box below username
      - Ready checkmark (when applicable)
      All elements properly aligned and spaced

3. Bottom Right - Ready/Start Area:
   Contains:
   - Button labeled "Start" or "Ready"
   - Countdown display (when applicable)
   - Proper spacing from edges

#### Interaction Behavior

1. Player Component Interactions:
   
   a) Drag and Drop:
      - Grab handle on player icon/name
      - Visual feedback during drag:
        * Slight scale up of dragged component
        * Shadow effect
        * Position preview at valid drop locations
      - Other players animate to new positions when:
        * Hovering valid swap location
        * After successful swap
      - Smooth animation back if invalid drop
      - Position swap animation using ellipse path

   b) Team Number Control:
      - Spin box with increment/decrement
      - Direct numeric input
      - Validation for team range
      - Updates reflected immediately
      - Disabled for non-host players

   c) Remove Button:
      - Only visible for host
      - Hover effect
      - Click animation
      - Confirmation if player is human
      - Smooth removal animation
      - Other players animate to new positions

2. Settings Panel Interactions:
   
   a) Controls:
      - All controls disabled for non-host
      - Real-time updates
      - Validation feedback
      - Network sync for multiplayer

   b) Add Player:
      - Shows loading state during add
      - Disabled when at capacity
      - Triggers position recalculation
      - Smooth animation for new player

3. Ready/Start System:

   Singleplayer:
   - Simple "Start" button
   - Immediate game start
   - Loading state during initialization

   Multiplayer:
   - "Ready" button toggles state
   - Shows checkmark when ready
   - Changes to "Unready" when ready
   - Countdown appears when all ready:
     * Centered near button
     * Large, clear numbers
     * Smooth transition between numbers
     * Cancellable by any player unreadying

#### State Management

1. Player Positions:
   ```rust
   ///This calculates the position the player should move towards.
   ///It does not calculate the position of the player for the next frame during movement animation.
   fn calculate_player_position(index: usize, total: usize) -> (f32, f32) {
       let theta = 2.0 * PI * index as f32 / total as f32 + 3.0 * PI / 2.0;
       (-theta.cos() * ELLIPSE_RADIUS, -ELLIPSE_HEIGHT_RATIO * theta.sin() * ELLIPSE_RADIUS)
   }
   ```

2. Animation State:
   - Track position transitions
   - Handle interrupted animations
   - Maintain smooth motion

3. Network State:
   - Track connection status
   - Handle disconnections
   - Sync player states
   - Ready status management

### Game Scene
#### Layout Structure
The scene represents the active gameplay area with several key regions:

1. Top Bar Controls:
   a) Settings Gear (Top Left):
      - Gear icon button (GEAR)
      - Subtle hover effect
      - Click opens popup with:
        * Modal overlay with dark background
        * Three buttons vertically arranged:
          - "Settings" (opens settings component)
          - "Leave Game" (shows confirmation)
          - "Resume" (closes popup)
        * All buttons with hover effects
        * Proper spacing and padding

2. Game Information Bar (Bottom):
   Full width bar containing:
   
   a) Round Information:
      - Round number ("Round X")
      - Clear, medium-sized text
      - Left-aligned in bar

   b) Team Scores:
      - Format: "Team X: [score] - Team Y: [score]"
      - Updates with animations
      - Centered in bar

   c) Trump and Lead Information:
      - Trump suit icon + text
      - Lead suit icon + text
      - Icons properly sized (32x32)
      - Right-aligned grouping

   d) Bid Information:
      - Highest bid display
      - Bidding player's name
      - Updates with animation

   e) Pile Owner:
      - Username of current pile owner
      - Updates each trick

3. Player Ellipse Layout:
   
   a) Configuration:
      - Same ellipse math as setup scene
      - Local player always at bottom
      - Other players distributed evenly
	  - Order of players should be maintained from setup
      - Radius: 525 units
      - Height ratio: 0.625

   b) Player Displays (Non-Local):
      Each contains:
      - Avatar (PERSON_ICON/BOT_ICON)
      - Username below
      - Team number chip
      - Face-down card display:
        * Shows number of cards
        * Slightly fanned arrangement
        * Updates as cards are played
      - Glow effect when active turn

   c) Local Player Area (Bottom):
      - No avatar displayed
      - Username and team number
      - Cards displayed face-up:
        * Horizontal arrangement
        * Slight overlap for space efficiency
        * Hover fans card out
        * Drag-drop enabled
        * Glow effect when playable

4. Center Play Area:
   
   a) Card Pile:
      - Central focus point
      - Shows last played card face-up
      - Previous cards slightly visible underneath
      - Clear drop target when dragging
      - Animated card additions

   b) Deck Display:
      - Positioned off-center
      - Shows remaining deck face-down
      - Updates count visually
      - Subtle animations for deals

#### Interaction Systems

1. Card Play Mechanics:
   
   a) Drag and Drop:
      - Cards lift on hover
      - Smooth drag animation
      - Valid/invalid drop feedback:
        * Green glow for valid
        * Red glow for invalid
        * Returns to hand if invalid
      - Snaps to pile when dropped
      - Proper z-index handling

   b) Click to Play:
      - Alternative to drag-drop
      - Highlights selected card
      - Confirms legal play
      - Animates to pile

   c) Keyboard Shortcuts:
      - Number keys 1-6 for cards
      - Visual feedback on press
      - Respects legal play rules

2. Bidding System:
   
   When it's player's turn to bid:
   - "Make Your Bid" prompt appears
   - Button row shows available bids:
     * "Pass" always shown
     * Numeric buttons (2-5) if valid
     * Dealer override (5) when applicable
   - Buttons highlight on hover
   - Selection animates and confirms
   - Invalid bids prevented
   - Keyboard shortcuts active

3. Turn Indication:
   
   a) Active Player:
      - Glowing avatar effect
      - Pulsing animation
      - Clear visual priority

   b) Local Player Turn:
      - Cards glow when playable
      - Invalid cards dimmed
      - Clear "Your Turn" indicator
      - Timer if applicable

4. Round Summary Display:
   
   After each round:
   - Modal overlay appears
   - Shows point breakdown:
     * High card points
     * Low card points
     * Jack points
     * Game points per team
   - Animated score updates
   - Continue button
   - Smooth transitions

5. Game End Sequence:
   
   When game concludes:
   - Final round summary
   - Overall game summary:
     * Final team scores
     * Winning team highlighted
     * Statistics if applicable
   - "Leave Game" button
   - Victory animations/effects

#### State Management

1. Game State Tracking:
   ```rust
   struct GameState {
       round: u32,
       scores: Vec<i32>,
       trump_suit: Option<Suit>,
       lead_suit: Option<Suit>,
       highest_bid: u8,
       pile_owner: String,
       active_player_index: u8,
       local_player_index: u8,
       hand: Vec<Card>,
       center_pile: Vec<Card>,
       player_card_counts: Vec<u8>
   }
   ```

2. Animation States:
   - Card movements
   - Score updates
   - Turn transitions
   - Prompt displays
   - Error messages

3. Network Integration:
   - Real-time updates
   - State synchronization
   - Disconnection handling
   - Reconnection logic

4. Error Handling:
   - Invalid plays
   - Connection issues
   - State mismatches
   - Recovery procedures

## State Management Architecture

### Game State Abstraction
The game uses a unified approach to handle both singleplayer and multiplayer modes, abstracting the differences between local and networked play.

1. Setup Phase:
   ```rust
   // GameBuilder manages setup state
   struct GameBuilder {
       settings: GameSettings,
       players: Vec<PlayerData>,
       // other setup fields...
   }
   ```
   - Used in both singleplayer and multiplayer
   - Contains mutable game configuration
   - Accessible fields for UI construction
   - Converted to GameRuntime on game start

2. Runtime Phase:
   ```rust
   // GameRuntime manages active game state
   struct GameRuntime {
       // Internal game state
       prompt_senders: Vec<(Sender<u8>, Sender<Card>)>,  // For bid and card prompts
       cancel_token: CancellationToken,
       // other runtime fields...
   }
   ```
   - Runs in separate async task
   - Emits GameEvents for state changes
   - Awaits player input through prompt channels
   - State is managed internally

### Event Flow

1. Singleplayer Mode:
   ```rust
   enum GameEvent {
       PlayerBid { index: u8, amount: u8 },
       PlayerPlayedCard { index: u8, card: Card },
       PlayerTurn { index: u8 },
       PromptBid { index: u8, hand: Vec<Card> },
       PromptCard { index: u8, hand: Vec<Card> },
       // other events...
   }
   ```
   - GameRuntime emits events directly
   - UI subscribes to events
   - Prompts handled through transmitters
   - State updates are immediate

2. Multiplayer Mode:
   ```rust
   enum Protocol {
       GameEvent(GameEvent),  // Same events as singleplayer
       UpdatePlayerTeam { index: u8, team: u8 },
       UpdatePlayerType { index: u8, human: bool },
       // other protocol messages...
   }
   ```
   - ServerConnection handles network communication
   - Same GameEvents wrapped in Protocol enum
   - Additional messages for multiplayer coordination
   - State updates through network messages

### Implementation Pattern

1. Setup Scene:
   ```rust
   match &self.host {
       Host::SingleplayerSetup(builder) => {
           // Access builder.settings, builder.players directly
           // Modify setup state through builder methods
       }
       Host::Host => {
           // Send setup changes through ServerConnection
           // Receive updates through Protocol messages
       }
       Host::Client => {
           // Read-only access to setup state
           // Cannot modify settings
       }
   }
   ```

2. Game Scene:
   ```rust
   match &self.host {
       Host::SingleplayerGame(runtime) => {
           // Listen for GameEvents
           // Send responses through prompt_senders
       }
       Host::Host | Host::Client => {
           // Listen for Protocol::GameEvent
           // Send responses through ServerConnection
       }
   }
   ```

3. Common Pattern:
   - Use same event types for both modes
   - Abstract input/output through common interfaces
   - Handle state updates consistently
   - Maintain single source of truth

### Network Integration

1. Server Communication:
   ```rust
   impl ServerConnection {
       // Sends game events to server
       pub async fn send_game_event(&mut self, event: GameEvent) -> bool;
       
       // Handles incoming messages
       async fn handle_message(&mut self, message: Protocol) {
           match message {
               Protocol::GameEvent(event) => {
                   // Handle same as singleplayer events
               }
               // Handle other protocol messages
           }
       }
   }
   ```

2. State Synchronization:
   - Server acts as source of truth
   - Local state updated through Protocol messages
   - Events flow through ServerConnection
   - Error handling for network issues

This architecture allows the UI to:
1. Handle both modes consistently
2. Use same components and layouts
3. Share event handling logic
4. Maintain clean separation of concerns

## Component Architecture

### Reusable Components
1. Card Component:
   ```rust
   pub struct Card {
       value: shared::Card,
       dragging: bool,
       playable: bool,
       position: Point,
       target: Option<Point>,
       // Animation state...
   }
   ```
   - Handles both face-up and face-down states
   - Manages drag-and-drop interactions
   - Handles animations between positions
   - Supports keyboard focus for accessibility

2. Player Display Component:
   - Reused in both Setup and Game scenes
   - Manages avatar, username, team display
   - Handles position animations
   - Supports different states (active, ready, etc.)

3. Modal Component:
   - Used for settings, prompts, confirmations
   - Consistent styling and behavior
   - Manages focus and keyboard navigation
   - Handles backdrop clicks and animations

### Animation System

1. Timing Constants:
   ```rust
   const CARD_MOVE_DURATION: Duration = Duration::from_millis(300);
   const PLAYER_MOVE_DURATION: Duration = Duration::from_millis(500);
   const SCORE_UPDATE_DURATION: Duration = Duration::from_millis(800);
   const FADE_DURATION: Duration = Duration::from_millis(200);
   ```

2. Easing Functions:
   ```rust
   fn ease_out_cubic(t: f32) -> f32 {
       1.0 - (1.0 - t).powi(3)
   }
   
   fn ease_out_elastic(t: f32) -> f32 {
       // For bouncy card animations
   }
   ```

3. Animation States:
   ```rust
   enum AnimationState {
       Idle,
       Moving { start: Point, end: Point, progress: f32 },
       Fading { start: f32, end: f32, progress: f32 },
       // Other states...
   }
   ```

### Error Recovery Flows

1. Network Disconnection:
   - Attempt reconnection up to 3 times
   - Show reconnecting overlay with progress
   - Maintain game state during reconnection
   - Resync with server on success
   - Return to main menu on failure

2. Game State Mismatch:
   - Detect inconsistencies in state
   - Request full state sync from server
   - Show loading overlay during sync
   - Log mismatch for debugging

3. Invalid Actions:
   - Show immediate visual feedback
   - Revert invalid state changes
   - Display user-friendly error messages
   - Provide recovery suggestions

### Testing Requirements

1. Component Tests:
   - Card interactions
   - Player display animations
   - Modal behaviors
   - Settings persistence

2. Scene Integration Tests:
   - Setup flow validation
   - Game state transitions
   - Multiplayer synchronization
   - Error handling scenarios

3. Performance Benchmarks:
   - Animation smoothness (60 FPS target)
   - State update latency (<16ms)
   - Memory usage (<100MB)
   - Network responsiveness (<100ms)

### Iced Subscriptions

1. Game Events:
   ```rust
   fn subscription(&self) -> Subscription<Message> {
       match &self.host {
           Host::SingleplayerGame(runtime) => {
               game_events_subscription(runtime)
           }
           Host::Host | Host::Client => {
               network_events_subscription(&self.connection)
           }
           _ => Subscription::none(),
       }
   }
   ```

2. Animation Updates:
   ```rust
   fn animation_subscription(&self) -> Subscription<Message> {
       if self.has_active_animations() {
           time::every(Duration::from_millis(16))
               .map(Message::AnimationTick)
       } else {
           Subscription::none()
       }
   }
   ```

### Event Structure
```rust
enum Event {
    // Scene Navigation
    TransitionTo(Scene),
    Back,
    
    // Game Events
    GameEvent(GameEvent),
    NetworkMessage(Protocol),
    
    // UI Events
    CardDragged { id: usize, position: Point },
    CardDropped { id: usize },
    PlayerDragged { index: usize, position: Point },
    PlayerDropped { index: usize },
    
    // Settings
    VolumeChanged(VolumeType, f32),
    KeybindChanged(KeybindType, String),
    
    // Animation
    AnimationTick(Instant),
    
    // Network
    ConnectionStateChanged(ConnectionState),
    
    // Error
    ErrorOccurred(Error),

	//possibly more variants
}
```
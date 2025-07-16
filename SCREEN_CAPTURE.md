# Screen Capture Feature

## Overview

The screen capture feature allows users to record periodic screenshots of the particle simulation for creating animations, time-lapse sequences, or documentation purposes.

## Features

### 1. Periodic Recording
- **Automatic Capture**: Set an interval (in seconds) for automatic periodic screenshots
- **Manual Capture**: Take immediate screenshots with the "Capture Now" button
- **Recording Status**: Visual feedback showing recording state and frame count

### 2. Region Selection
- **Full Screen**: Capture the entire simulation window (default)
- **Custom Region**: Select a specific rectangular area to capture
- **Interactive Selection**: Click and drag to define capture region
- **Visual Feedback**: Yellow rectangle shows current selection, green rectangle shows confirmed region

### 3. File Management
- **Automatic Naming**: Files are named with timestamp and frame counter (e.g., `capture_20250716_143022_0001.png`)
- **Custom Folder**: Specify output directory for saved images
- **Directory Creation**: Automatically creates capture folder if it doesn't exist

## User Interface

### Screen Capture Tab
The screen capture functionality is accessible through the "📷 Screen Capture" tab in the main control panel.

#### Controls
- **Start/Stop Recording**: Toggle periodic capture on/off
- **Capture Now**: Take an immediate screenshot
- **Capture Interval**: Set time between automatic captures (0.1-10.0 seconds)
- **Save Folder**: Specify output directory

#### Region Selection
- **Select Region**: Enter region selection mode
- **Full Screen**: Reset to capture entire window
- **Clear Region**: Remove current region selection

#### Statistics
- **Total Captures**: Count of images saved in current session
- **Last Capture**: Time since last screenshot
- **Next Capture**: Countdown to next automatic capture

## Usage Instructions

### Basic Recording
1. Open the control panel with the 'E' key
2. Navigate to the "📷 Screen Capture" tab
3. Optionally set a capture folder name
4. Click "🔴 Start Recording" to begin periodic capture
5. Click "⏹️ Stop Recording" to end

### Region Selection
1. Click "🎯 Select Region" to enter selection mode
2. Click and drag on the simulation view to define the capture area
3. The selected region will be highlighted in yellow during selection
4. Release the mouse to confirm the region (turns green)
5. Right-click or press Escape to cancel selection

### Manual Capture
- Click "📷 Capture Now" at any time to take an immediate screenshot
- This works regardless of recording status

## Technical Implementation

### Architecture
- **Screen Capture Manager**: Handles timing, file operations, and region management
- **GUI Integration**: Screen capture tab in the main control panel
- **Input Handling**: Mouse input for region selection
- **Rendering Integration**: Visual feedback for selected regions

### File Format
- **Format**: PNG images with RGBA channels
- **Naming**: `capture_YYYYMMDD_HHMMSS_NNNN.png`
- **Location**: Configurable output directory (default: "captures")

### Dependencies
- **image**: Image processing and file I/O
- **chrono**: Timestamp generation for filenames

## Configuration

### Default Settings
- **Capture Interval**: 1.0 seconds
- **Output Folder**: "captures"
- **Capture Region**: Full screen
- **Recording State**: Disabled

### Customizable Options
- Capture interval (0.1 to 10.0 seconds via GUI)
- Output folder path
- Capture region (full screen or custom rectangle)

## Keyboard Shortcuts

- **E**: Toggle control panel (to access screen capture tab)
- **Escape**: Cancel region selection
- **Right Click**: Cancel region selection

## Tips and Best Practices

### For Animations
1. Set a consistent capture interval (e.g., 0.5 seconds)
2. Use a specific region to focus on interesting simulation areas
3. Keep the simulation running smoothly for best results

### For Documentation
1. Use "Capture Now" for specific moments
2. Select regions to highlight particular phenomena
3. Use descriptive folder names for organization

### Performance Considerations
- Lower capture intervals increase file output frequency
- Smaller regions reduce file sizes
- Consider disk space when running long captures

## Troubleshooting

### Common Issues
1. **Folder Creation Fails**: Check write permissions for the specified directory
2. **No Images Saved**: Ensure recording is enabled and interval has elapsed
3. **Region Selection Not Working**: Make sure you're in selection mode

### File Locations
- Default capture folder: `./captures/` (relative to executable)
- Custom folders: As specified in the "Save Folder" field

## Future Enhancements

### Planned Features
- **Video Export**: Convert image sequences to video files
- **Compression Options**: JPEG support for smaller file sizes
- **Batch Processing**: Tools for managing large numbers of captures
- **Real-time Preview**: Thumbnail preview of capture region

### API Extensions
- **Command Line Interface**: Scriptable capture control
- **External Triggers**: Capture based on simulation events
- **Custom Formats**: Support for additional image formats

## Integration Notes

The screen capture system is designed to integrate seamlessly with the existing particle simulation:

- **Minimal Performance Impact**: Capture operations are optimized to not interfere with simulation
- **State Preservation**: Recording settings persist during simulation runs
- **GUI Integration**: Native integration with the existing tab-based interface
- **Input Handling**: Careful coordination with existing mouse/keyboard controls

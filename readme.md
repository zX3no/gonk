## Redesign Doc

User <--> UI <--> Player <--> Backend <--> OS

There is also the indexer, which the UI and Player will need to talk to.

### UI 

- Input
    - Keyboard
        - Browser
            - Up
            - Down
            - Next Browser Mode
            - Previous Browser Mode
            - Mode 
                - Artist
                    - Add artist to queue
                - Album
                    - Add album to queue
                - Song
                    - Add song to queue
        - Queue
            - Up
            - Down
            - Play selected
            - Remove selected
            - Remove all
        - Search
            - Enter query
            - Delete query
            - Exit to previous mode
    - Mouse
        - Browser 
            - Select item
            - Scroll items
            - Mode 
                - Artist
                    - Next Mode
                - Album
                    - Next Mode
                - Song
                    - Add song to queue
        - Queue
            - Scroll items
            - Select item
            - Play selected
        - Track Seeker
            - Seek based on position


- Mode
    - Browser
        - Artist
            - Selection
        - Album
            - Selection
        - Song
            - Selection
    - Queue
        - Playing
        - Selection
    - Track Seeker 
        - Duration
        - Elapsed
    - Search
        - Query


There are two modes, the UI mode and the Browser mode.
The browser mode is either, artist, album or song.
The UI mode is either browser, queue, seeker or search.

The browser and queue need to have different modes.
So you can move up and down in each of them.
Each mode needs to have it's own independent state.

### Player 

- Player
    - Queue(list of songs)
    - Playing Song(optional) 
    - Index(optional)
    - Volume
    - Backend
    - Event
    
- Song 
    - Title
    - Album
    - Artist
    - Path
    - Track Number
    - Duration

- Event 
    - Next
    - Previous
    - Volume Up
    - Volume Down
    - Play
    - Pause
    - Stop

- Loop 
    - Song playing?
        - Song finished?
            - Play next song
    - Any events?
        - Do event

The player is always running in a loop.

### Backend 

- Context 
- Handle

The context is an instance of soloud.
The handle is a way to access the currently playing song.

What are the steps to playing an audio file?

Start the audio stream

Set-up symphonia to decode packets. 
- This is very unreliable
- Maybe I should change to claxon?

Send packets to the audio stream.


There is no reason really why I can't do this on the same thread?

What needs to shared across threads?

- Elapsed
- Duration
- Volume
- Track gain?
- State (Playing/Paused/Stopped)
- The next song to play
- Decoded samples (We can do this on the playback thread now)
- The desired seek position

How can we share across thread boundries?
- Message passing (not very fast)
- Arc<Mutex<T>> (fast, but not very ergonomic)
- static mut T (very fast, slightly better ergonomics) 
- static mut T does not work well for sending events
- There should probably be a combination of both message passing and static mut.
- Rainout uses a ringbuffer to send events across threads.

What will be global
- Elapsed
- Duration
- Volume
- State (Playing/Paused/Stopped)

What will be send through messages
- Seeking
- The next song
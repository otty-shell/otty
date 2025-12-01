Implementing Blocks

The reason you don’t see a feature like blocks (with the exception of Upterm) in most other terminals is because the terminal has no concept of what program is running, or really of anything that’s happening within the shell. At a high level, a terminal reads and writes bytes from a pseudoterminal to interact with the shell. This technology is very antiquated--the shell essentially thinks it is interacting with a physical teletype terminal even though they haven’t been used in practice in over 30 years!

A terminal implements the VT100 spec, which is used to communicate information from the shell to the terminal. For example if the shell wants to tell the terminal to render text red, bold, and underline it would send the following escape code, which the terminal then parses and renders with the appropriate styles:

\033[31;1;4mHello\033[0m

Run in Warp
https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences covers how colors are represented via escape sequences very well. It’s also just generally one of my favorite stackoverflow answers--make sure to read the interlude!

Given all the terminal sees is characters in and characters out, trying to determine when a command is run is next to impossible using just the characters from the pseudoterminal.

Thankfully, most shells provide hooks for before the prompt is rendered (zsh calls this precmd) and before a command is executed (preexec). Zsh and Fish have built in support for these hooks. Though bash does not have built in support for these hooks, scripts like bash-preexec exist to mimic the behavior of other shells. Using these hooks, we send a custom DCS (Device Control String) from the running session to Warp. This DCS contains an encoded JSON string that includes metadata about the session that we want to render. Within Warp we can parse the DCS, deserialize the JSON, and create a new block within our data model.

Data Model
The VT100 spec represents the viewport in terms of rows and columns which naturally leads to most terminals using a grid as their internal data model. We started by forking Alacritty’s model code, which was well suited for us because it was already written in Rust and is tailored to be as performant as possible. Nathan Lilienthal and Joe Wilm, two of the maintainers of Alacritty, were early supporters of Warp and were extremely gracious in reviewing early design docs and providing their technical expertise.

We quickly realized, however, that a single grid would not work for building a feature like blocks: separating commands from each other requires ensuring that terminal output is actually written on separate lines of a grid and can’t be overwritten by output from another command. Neither of these can be guaranteed by the VT100 spec. For example, consider a command like

bash-5.1$ printf "hello"
hellobash-5.1$

Run in Warp
‍

Here printf "hello" output the text "hello", but no newline was inserted, so the prompt for the next command was inserted on the same line. Similarly, escape sequences exist that allow moving the cursor up within the visible screen by n rows, allowing any command to overwrite any command that was already executed. With a traditional grid data model, we wouldn’t be able to support the case where a single line had the command from one block and the output from another. We also didn’t want the output from one command to be able to overwrite the contents from a previous block.

To fix these issues, we create a separate grid for each command and output based on the precmd/preexec hooks we receive from the shell. This level of grid isolation ensures we can separate commands and their output without having to deal with the output of one command overwriting that of another. We found this approach to be a good balance of being faithful to the VT100 spec and using the existing Alacritty grid code with rendering blocks in a more flexible way that was traditionally prescribed by a teletype terminal. This model gives us more flexibility for implementing features for a single block that are traditionally for an entire terminal grid. For example, searching per block and copying the command/output separately are trivial to implement if our model separates each command and output into their own submodel.

Input Editor

‍

Together with Nathan Sobo, we built a full-fledged text editor as our input editor as an editor view in our framework. Building this as a full text editor unblocked features like multiple cursors and selections within the editor and also will let us use the editor as the default editor within the shell if we ever want to support it.

Because we built our own command line editor, we had to re-implement all the existing shortcuts users are used to while editing (such as up arrow, ctrl-r, and tab completion) while also balancing implementing new modern shortcuts like Move Cursor By Word or Select All that previously didn’t exist in the shell line editor. We built an action dispatching system into our framework that can be triggered based on keybindings so implementing all these shortcuts was relatively trivial. This also made it easy to enable/disable various shortcuts depending on the state of the app: for example “ctrl-r” for searching previous commands should only be enabled if the input editor is visible.

Building the editor also heavily relied on a SumTree, a custom data structure that is essentially a Rope that can hold generic types and can index the type on multiple dimensions.

‍

‍


‍

‍

Like many text editors, we use the SumTree to hold the contents of our buffer. We also use the SumTree for transformations of the editor that don’t affect the underlying buffer contents (such as code folding, holding unselectable and uneditable annotations, etc). Representing transformations as a SumTree allows us to efficiently query for both the buffer text and the display text at any given point in the editor. We also use SumTrees to keep track of the operations (or edits) on the buffer. We intentionally designed our editor to be an Operation-based CRDT from the start, so that we can support real-time collaboration within the editor.

Looking Forward
Looking forward, our stack opens up the ability to realize our full vision of what Warp can be. By choosing Rust and building a UI framework that is rendering-platform agnostic, we can easily scale to other platforms and even the web by compiling to WASM (a mistake we made was to not think about which crates will and won’t compile to WASM early on) and rendering with WebGL.

One feature that the web rendering enables is real-time collaboration. You can share a secure link to your teammate to debug issues in real-time https://github.com/warpdotdev/warp/issues/2.

We can also leverage the “Blocks” model to enable users to search history by more than just the command: https://github.com/warpdotdev/warp/issues/1.

Our precmd and preexec hooks also provide the necessary technical foundation for Environment variable sharing: https://github.com/warpdotdev/warp/issues/3.

* Edit: When we published this post, we had not set up our Github issues yet. We have since transferred the technical implementation details to Github.

Conclusion
Performance is one of our most important features--it’s a feature most users won’t notice on a day-to-day basis but it makes a huge difference in building a quality product for the end user.

If you’re interested in learning more, please check out our site and request early access to our beta here.  We are still in the early days of implementing our vision but would love your feedback and input as we move forward!
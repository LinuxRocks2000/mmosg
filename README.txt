This is an MMO version of Strategy Game. It is a large, fixed-size game board with basically the same premise as the original, but instead of dumb enemies you fight other players. You get points for shooting enemy ships, and you get lots of points for shooting down enemy castles. You buy new ships with points. It's last-man-standing battle royale, so the last living player wins. Or, if there's a timer (configurable), whoever has the most points at the end wins.

Written with CrowCPP - you'll have to install a latest version before you can compile. (Of course, you can use a prebuilt binary, of which there is exactly one).

Once you've compiled the server (hint: it requires c++20 and Crow installed properly), run build.sh. This won't work on Windows. Then run the binary built - it's probably just going to be server-Linux-x86_64, unless you're on something else. MacOS might work.

Then, share the password it spits out with people. Or don't. You can also run "change password", and it'll prompt you for a new password to use.

When everyone's joined and ready, run "start".
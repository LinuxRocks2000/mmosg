/* By Tyler Clarke, circa April 2023 a.d.
    The "front page" screen for MMOSG.
    If you edit, *please* don't do that thing where you strip out my name (Tyler Clarke) and put your own in.
*/


const char* STARTING_SCREEN =
"\033[91m"
"          #######################      #######################     ##################    ##############    ##############\n"
"        ########################     ########################    ###################   ##############    ###############\n"
"      #####     #####     #####    #####     #####     #####   #####          #####   #####             #####\n"
"     #####     #####     #####    #####     #####     #####   #####          #####    ############     #####\n"
"    #####     #####     #####    #####     #####     #####   #####          #####     ############    #####    #######\n"
"   #####               #####    #####               #####   #####          #####            #####    #####      #####\n"
"  #####               #####    #####               #####   ###################     #############    ################\n"
" #####               ####     #####               ####    ##################     #############     ###############\033[0m\n"
"\n"
"┌───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐\n"
"│                                                                                                                   │\n"
"│                                                                                                                   │\n"
"│                                  \033[1;91mMassively Multiplayer Online Strategy Game\033[0m                                       │\n"
"│                                                                                                                   │\n"
"│                                              \033[5;32mBy Tyler Clarke\033[0m                                                      │\n"
"│                                                                                                                   │\n"
"│       Binds an HTTP + websocket server for the game to port 3000, or whatever you specify on command-line.        │\n"
"│                                                                                                                   │\n"
"└───────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘\n"
"\n"
"\033[1;34m While also being used for broadcasting informative messages, this terminal will accept several input commands:\033[0m\n"
"  \033[34m• start:\033[0m Lock out new arrivals (even if they get the right code, they will be put in Spectator mode), and begin play.\n"
"  \033[34m• data:\033[0m Print statistics about the currently running game.\n"
"  \033[34m• change password:\033[0m Change the entry password. People already logged in will not be affected. After running this, you will be prompted for the new password.\n"
"  \033[34m• broadcast:\033[0m Broadcast a message to all current players. It will be shown as an alert box.\n"
"  \033[34m• resize:\033[0m Change the size of the gameboard.\n"
"  \033[34m• reset:\033[0m Clear the gameboard, drop all clients, purge all banners, and set the counter back to initial strategy phase.\n"
"  \033[34m• quit/exit:\033[0m Kill the server. Ctrl + C works too.\n"
"  \033[34m• drop client:\033[0m You will be prompted for a client number (from the data command) to delete. They will be disconnected and all their gamepieces purged.\n"
"  \033[34m• skip stage:\033[0m Go to the next stage - if in strategy, go to play, if in play, go to strategy.\n"
"  \033[34m• clear unowned:\033[0m Delete all pieces without an owner. Useful for dealing with errant disconnections.\n"
"\n"
"\033[1;33m************** Tyler is looking for a job! If you need some code done, contact him at plupy44@gmail.com *************\033[0m\n";
#include <fcntl.h>
#include <termios.h>
#include <iostream>
#include <cassert>
#include <string>
#include <vector>


class CleverTerminal {
    std::vector <std::string> headLines;
    std::string outBuffer;
    bool needsUpdate = true;
    struct termios canonicalTio, noncanonicalTio;
    bool isCanonical = true;
    std::string curLine;
    bool gotNewline = false;
    int escape = 0;
    bool inited = false;
public:
    std::string prompt = "> ";

    std::string& operator[](size_t index){
        return headLines[index];
    }

    void printLn(std::string line){
        line += "\n";
        outBuffer += line;
        needsUpdate = true;
    }

    void printErr(std::string line){
        printLn("\033[91mERROR: " + line + "\033[0m");
    }

    void init(int numHeadLines){
        inited = true;
        for (size_t i = 0; i < numHeadLines; i ++){
            std::cout << std::endl;
            headLines.push_back("");
        }
        fcntl(fileno(stdin), F_SETFL, O_NONBLOCK | fcntl(fileno(stdin), F_GETFL, NULL));
        tcgetattr(fileno(stdin), &canonicalTio);
        noncanonicalTio = canonicalTio;
        noncanonicalTio.c_lflag &= ~ICANON;
        noncanonicalTio.c_lflag &= ~ECHO;
    }

    void canonical(){ // If you call _this_, it's understood that you're handling inputs otherwise
        assert(inited);
        isCanonical = true;
        tcsetattr(fileno(stdin), TCSANOW, &canonicalTio);
    }

    void noncanonical(){ // If you call this, it's understood that you'll be getting keyboard data through us
        assert(inited);
        isCanonical = false;
        tcsetattr(fileno(stdin), TCSANOW, &noncanonicalTio);
    }

    size_t countLines(std::string lines){
        size_t ret = 0;
        for (char c : lines){
            if (c == 10){
                ret ++;
            }
        }
        return ret;
    }

    void update(){
        size_t outLines = countLines(outBuffer);
        for (size_t i = 0; i < outLines; i ++){ // Add new lines to accommodate for the out buffer
            std::cout << std::endl;
        }
        for (size_t i = 0; i < headLines.size() + outLines; i ++){ // Go up to the start of the frontpanel
            std::cout << "\033[2K\033[F\033[2K";
        }
        std::cout << outBuffer; // Print the new lines that have been injected
        for (size_t i = 0; i < headLines.size(); i ++){ // Print the header lines
            std::cout << headLines[i] << std::endl;
        }
        if (!isCanonical){
            std::cout << prompt << curLine << std::flush;
        }
        outBuffer = "";
        if (!isCanonical) {
            char chr = getchar();
            if (chr != EOF){
                if (chr == 127){
                    if (curLine.size() > 0){
                        curLine.pop_back();
                        needsUpdate = true;
                    }
                }
                else if (chr == 10){
                    gotNewline = true;
                }
                else if (chr == 27){
                    escape = 1;
                }
                else if (escape == 1){
                    if (chr == 91){
                        escape = 2;
                    }
                }
                else if (escape == 2){
                    escape = 0; // This program is not yet equipped to deal with cursor movement
                }
                else{
                    curLine += chr;
                    needsUpdate = true;
                }
            }
        }
    }

    bool hasLine(){
        return gotNewline;
    }

    std::string getLine(){
        if (gotNewline){
            std::string oldLine = curLine;
            curLine = "";
            gotNewline = false;
            return oldLine;
        }
        else {
            return "";
        }
    }

    std::string waitForLine() {
        while (!hasLine()){
            update();
            usleep(50000);
        }
        return getLine();
    }

    std::string input(std::string pr){
        std::string oldPrompt = prompt;
        prompt = pr;
        std::string ret = waitForLine();
        prompt = oldPrompt;
        return ret;
    }
};
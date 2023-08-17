#include "terminalstuff.hpp"
#include "logo.h"
#include <iostream>
#include <unistd.h>
#include <termios.h>
#include <fcntl.h>
#include "database.hpp"


int main(){
    _database::create("logins.dat");
    _database database("logins.dat");
    _db_object stling = { "STRING" };
    _db_object llRoot {
        database.pushDbObject(stling),
        0
    };
    database.pushDbObject(llRoot);
    database.linkedListPush(llRoot, { 609L });
    _db_object sixohnine = database.linkedListGet(llRoot, 1);
    std::cout << sixohnine.longNum << std::endl;
    return 0;
}

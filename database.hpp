// The goal with this file is to operate the user database, including login data and whatnot
#include <stdio.h>
#include <atomic>
#include <string>
#include <stdexcept>
#include <cstring>
#include <cassert>
/* The Floobity format: a binary format designed to maximize database efficiency.
    The first 2 bytes are always "TC". This also means any pointers less than 2 are invalid.

    Entirely composed of data chunks. First byte of a chunk is the type - for 0, the next 8 bytes are a 64-bit signed int, and for 1 are a 64-bit double. 2 is a string. For strings, the next byte is the chunk size - if == 256, then after the data the next chunk is expected to be a continuation of that string (size 0 for the next chunk is perfectly valid, if the string was exactly 256 bytes).
    Type 3 (array) is basically a statically-sized Python list. The first byte is the number of 64-bit pointers (byte numbers) referencing the chunks containing the values of the array. Array sizes are fixed.
    Type 4 is a linked list element. The first 8 bytes are a pointer to the next element (0, if it's the end of the list), and then the next 8 bytes after that are a pointer to the chunk.
    If the first bit is set (the number is >128), the data has been "deleted" and should be fully removed when defragged.
    Type 5 is a reference - the next 8 bytes are a pointer to a position in the file.
    Type 6 is a char, also usable as a boolean. The next 1 byte is the character value.

    Most data is immutable. Instead of deleting data, the reference that holds the data is just overwritten with a new reference to newly appended data (because appending is far less expensive than moving).
    The "deleted" data's first type bit is set - 128 is added to it - which signals that, to a defragger or processor, it should be ignored.

    The first chunk is considered the "leading label". It begins the tree.

    The first bit of every type is whether it's deleted or not. The second bit is reserved for future use. This limits the number of possible types to 64, which is more than enough.
*/

typedef uint64_t database_pointer_t;


enum DatabaseType : int {
    LONG        = 0,
    DOUBLE      = 1,
    STRING      = 2,
    ARRAY       = 3,
    LINKED_LIST = 4,
    REFERENCE   = 5,
    CHARACTER   = 6,
    NONE        = 255 // Invalid type + marked for delete
};


struct dbarray {
    dbarray(size_t size){
        _size = size;
        _array = (database_pointer_t*)malloc(sizeof(database_pointer_t) * size);
    }
    ~dbarray(){
        free(_array);
    }
    database_pointer_t* _array;
    size_t _size;
    database_pointer_t& operator[](size_t index){
        assert(index < _size);
        return _array[index];
    }
};


struct _db_object {
    bool copied = false;

    _db_object(){

    }

    _db_object(_db_object& parent) {
        type = parent.type;
        mRef = parent.mRef;
        refNext = parent.refNext;
        string = parent.string;
        longNum = parent.longNum;
        doubleNum = parent.doubleNum;
        character = parent.character;
        reference = parent.reference;
        parent.copied = true;
        std::cout << "copy." << std::endl;
    }

    _db_object(long _longNum){
        longNum = _longNum;
        type = LONG;
    }

    _db_object(double _doubleNum){
        type = DOUBLE;
        doubleNum = _doubleNum;
    }

    _db_object(char* s) {
        type = STRING;
        string = s;
    }

    _db_object(unsigned char s){
        type = CHARACTER;
        character = s;
    }

    _db_object(const char* s){
        type = STRING;
        unsigned long stringSize = strlen(s);
        string = (char*)malloc(stringSize + 1); // expects it to be a C string with a null terminating character
        memcpy(string, s, stringSize + 1);
        string[stringSize] = 0;
    }

    _db_object(database_pointer_t refTo){
        reference = refTo;
        type = REFERENCE;
    }

    _db_object(size_t arrsize, std::vector<database_pointer_t> init){
        type = ARRAY;
        array = new dbarray(arrsize);
        for (size_t i = 0; i < init.size(); i ++){
            (*array)[i] = init[i];
        }
    }

    _db_object(database_pointer_t ref, database_pointer_t next) {
        type = LINKED_LIST;
        reference = ref;
        refNext = next;
    }

    ~_db_object(){
        if (!copied){
            freeMemory();
        }
    }

    void freeMemory(){
        if (type == STRING){
            free(string);
        }
        else if (type == ARRAY){
            delete array;
        }
    }

    DatabaseType type;
    database_pointer_t mRef; // reference to this
    FILE* file;
    union {
        char* string;
        double doubleNum;
        long longNum;
        unsigned char character;
        database_pointer_t reference; // Reference to an enclosed item
        dbarray* array;
    };
    database_pointer_t refNext = 0; // Reference to next for linked lists

    void write(){ // BE CAREFUL: This will damage other chunks if used improperly.
        std::cout << "My REFERENCE IS" << mRef << std::endl;
        fseek(file, mRef, SEEK_SET);
        fputc((int)type, file);
        if (type == LONG){
            fwrite(&longNum, 8, 1, file);
        }
        else if (type == DOUBLE){
            fwrite(&doubleNum, 8, 1, file);
        }
        else if (type == STRING){
            unsigned long strSize = strlen(string);
            fwrite(&strSize, 8, 1, file);
            fwrite(string, 1, strSize, file);
        }
        else if (type == REFERENCE) {
            fwrite(&reference, 8, 1, file);
        }
        else if (type == ARRAY){
            fwrite(&array -> _size, 8, 1, file);
            for (size_t i = 0; i < array -> _size; i ++){
                fwrite(&((*array)[i]), 8, 1, file);
            }
        }
        else if (type == CHARACTER){
            fwrite(&character, 1, 1, file);
        }
        else if (type == LINKED_LIST){
            fwrite(&reference, 8, 1, file);
            fwrite(&refNext, 8, 1, file);
        }
    }

    void markDelete(){
        fseek(file, mRef, SEEK_SET);
        fputc((int)type | 128, file); // set the kill bit
    }

    unsigned long size(){
        if ((type == LONG) || (type == DOUBLE)){
            return 9;
        }
        if (type == STRING){
            return 9 + strlen(string);
        }
        if (type == REFERENCE){
            return 9;
        }
        if (type == ARRAY){
            return 9 + 8 * array -> _size;
        }
        std::cout << "BUG" << std::endl;
        return -1;
    }
};


class _database {
    FILE* file;
public:
    _database(const char* filename){
        _database::createIfNotExists(filename);
        file = fopen(filename, "r+");
        if ((fgetc(file) != 'T') || (fgetc(file) != 'C')) {
            std::cout << "Corrupt database file." << std::endl;
        }
    }

    _db_object _loadFromDisk(database_pointer_t reference){
        _db_object resolvedObj;
        resolvedObj.mRef = reference;
        resolvedObj.file = file;
        fseek(file, reference, SEEK_SET);
        resolvedObj.type = (DatabaseType)fgetc(file);
        switch ((DatabaseType)((int)resolvedObj.type & 63)) { // & 63 = unset last two bits, both of which are used for other purposes.
            case LONG:
                fread(&resolvedObj.longNum, 8, 1, file);
                break;
            case DOUBLE:
                fread(&resolvedObj.doubleNum, 8, 1, file);
                break;
            case STRING:
                unsigned long stringSize;
                fread(&stringSize, 8, 1, file);
                resolvedObj.string = (char*)malloc(stringSize);
                fread(resolvedObj.string, 1, stringSize, file);
                break;
            case REFERENCE:
                fread(&resolvedObj.reference, 8, 1, file);
                break;
            case ARRAY:
                size_t size;
                fread(&size, 8, 1, file);
                resolvedObj.array = new dbarray(size);
                for (size_t i = 0; i < size; i ++){
                    database_pointer_t reference;
                    fread(&reference, 8, 1, file);
                    (*resolvedObj.array)[i] = reference;
                }
                break;
            case CHARACTER:
                fread(&resolvedObj.character, 1, 1, file);
                break;
            case LINKED_LIST:
                fread(&resolvedObj.reference, 8, 1, file);
                fread(&resolvedObj.refNext, 8, 1, file);
                break;
        }
        return resolvedObj;
    }

    _db_object resolve(database_pointer_t reference){
        _db_object resolvedObj = _loadFromDisk(reference);
        while (resolvedObj.type == REFERENCE){
            resolvedObj = _loadFromDisk(resolvedObj.reference);
        }
        return resolvedObj;
    }

    database_pointer_t pushDbObject(_db_object& obj){
        fseek(file, 0, SEEK_END);
        obj.mRef = ftell(file);
        obj.file = file;
        obj.write();
        return obj.mRef;
    }

    static void create(const char* filename){
        FILE* nf = fopen(filename, "w+");
        fputc('T', nf);
        fputc('C', nf);
        fclose(nf);
    }

    static void createIfNotExists(const char* filename){
        if (access(filename, F_OK) != 0){
            create(filename);
        }
    }

    database_pointer_t getChunkReference(unsigned long number){ // Warning: SLOW! Should only be used to get handles at the start!
        database_pointer_t currentReference = 2;
        unsigned long count = 0;
        while (count < number){
            count ++;
            _db_object obj = resolve(currentReference);
            currentReference += obj.size();
        }
        return currentReference;
    }

    _db_object getLLHead(_db_object ll){
        _db_object current = ll;
        while (current.refNext != 0){
            current = _loadFromDisk(current.refNext);
        }
        return current;
    }

    database_pointer_t linkedListPush(_db_object ll, database_pointer_t item){
        ll = getLLHead(ll);
        _db_object ref { item, 0 };
        pushDbObject(ref);
        ll.refNext = ref.mRef;
        ll.write();
        return ref.mRef;
    }

    _db_object linkedListGet(_db_object ll, unsigned long index){
        _db_object rt = ll;
        for (unsigned long i = 0; i < index; i ++){
            rt = _loadFromDisk(rt.refNext);
        }
        return rt;
    }
};
#pragma once

#ifdef HOST
#define substring(...) substr(__VA_ARGS__)
#define indexOf(...) find(__VA_ARGS__)
#define isEmpty(...) empty(__VA_ARGS__)
#endif

#include <string>

typedef std::string String;

void delay(int);

class HardwareSerial {
 public:
  void print(String);
  void println(String);
};

extern HardwareSerial Serial;

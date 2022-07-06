constexpr int SIGNAL_PIN = A0;
constexpr int BAUD_RATE = 9600;

void setup() { Serial.begin(BAUD_RATE, SERIAL_8E1); }

void loop() {
  unsigned short value = analogRead(SIGNAL_PIN);
  Serial.write(reinterpret_cast<const char *>(&value), sizeof(unsigned short));
  delay(3);
}
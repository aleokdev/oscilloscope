constexpr int SIGNAL_PIN = A0;
constexpr int BAUD_RATE = 9600;

void setup() { Serial.begin(BAUD_RATE); }

void loop() {
  Serial.print(analogRead(SIGNAL_PIN));
  Serial.write(',');
  delay(2);
}
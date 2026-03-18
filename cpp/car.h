// This file has been generated from car.oml
#ifndef CAR_H
#define CAR_H

#include <cstdint>
#include <string>
#include <optional>
#include <utility>

class Engine {
public:
	Engine() = default;
	Engine(int64_t fuel, float hp) : fuel(std::move(fuel)), hp(std::move(hp)) {}

	Engine(const Engine& other) = default;
	Engine(Engine&& other) noexcept = default;
	Engine& operator=(const Engine& other) = default;
	Engine& operator=(Engine&& other) noexcept = default;
	~Engine() = default;

	int64_t getFuel() const { return fuel; }
	float getHp() const { return hp; }

	void setFuel(const int64_t& value) { fuel = value; }
	void setHp(const float& value) { hp = value; }
private:
	int64_t fuel;
	float hp;
};

class Car {
public:
	Car() = default;
	Car(Engine e1) : e1(std::move(e1)) {}

	Car(const Car& other) = default;
	Car(Car&& other) noexcept = default;
	Car& operator=(const Car& other) = default;
	Car& operator=(Car&& other) noexcept = default;
	~Car() = default;

	Engine getE1() const { return e1; }

	void setE1(const Engine& value) { e1 = value; }
private:
	Engine e1;
};
#endif // CAR_H


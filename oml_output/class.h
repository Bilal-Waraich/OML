// This file has been generated from class.oml
#ifndef CLASS_H
#define CLASS_H

#include <cstdint>
#include <string>
#include <optional>
#include <utility>

class Foo {
public:
	Foo() = default;
	Foo(int64_t meow, std::string hello, bool isTrue)
		: meow(std::move(meow)), hello(std::move(hello)), isTrue(std::move(isTrue)) {}

	Foo(const Foo& other) = default;
	Foo(Foo&& other) noexcept = default;
	Foo& operator=(const Foo& other) = default;
	Foo& operator=(Foo&& other) noexcept = default;
	~Foo() = default;

	int64_t getMeow() const { return meow; }
	std::string getHello() const { return hello; }
	bool getIsTrue() const { return isTrue; }

	void setHello(const std::string& value) { hello = value; }
	void setIsTrue(const bool& value) { isTrue = value; }
private:
	const int64_t meow;
	std::string hello;
	bool isTrue;
};

class Foo2 {
public:
	Foo2() = default;
	Foo2(int64_t meow, std::string hello, bool isTrue)
		: meow(std::move(meow)), hello(std::move(hello)), isTrue(std::move(isTrue)) {}

	Foo2(const Foo2& other) = default;
	Foo2(Foo2&& other) noexcept = default;
	Foo2& operator=(const Foo2& other) = default;
	Foo2& operator=(Foo2&& other) noexcept = default;
	~Foo2() = default;

	int64_t getMeow() const { return meow; }
	std::string getHello() const { return hello; }
	bool getIsTrue() const { return isTrue; }

	void setHello(const std::string& value) { hello = value; }
	void setIsTrue(const bool& value) { isTrue = value; }
private:
	const int64_t meow;
	std::string hello;
	bool isTrue;
};
#endif // CLASS_H


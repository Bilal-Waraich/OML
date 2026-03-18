// This file has been generated from student.oml
#ifndef STUDENT_H
#define STUDENT_H

#include <cstdint>
#include <string>
#include <optional>
#include <utility>
#include <array>
#include <vector>

class Student {
public:
	Student() = default;
	explicit Student(std::string name, uint32_t id, std::array<float, 5> grades, std::vector<std::string> courses, std::vector<std::string> notes)
		: name(std::move(name))
		, id(std::move(id))
		, grades(std::move(grades))
		, courses(std::move(courses))
		, notes(std::move(notes))
	{}
	Student(std::string name, uint32_t id, std::array<float, 5> grades, std::vector<std::string> courses, std::vector<std::string> notes, std::optional<std::string> advisor)
		: name(std::move(name))
		, id(std::move(id))
		, grades(std::move(grades))
		, courses(std::move(courses))
		, notes(std::move(notes))
		, advisor(std::move(advisor))
	{}

	Student(const Student& other) = default;
	Student(Student&& other) noexcept = default;
	Student& operator=(const Student& other) = default;
	Student& operator=(Student&& other) noexcept = default;
	~Student() = default;

	std::string name;
	uint32_t id;
	std::array<float, 5> grades;
	std::vector<std::string> courses;
	std::vector<std::string> notes;
	std::optional<std::string> advisor;
};

class Classroom {
public:
	Classroom() = default;
	Classroom(std::string room_code, std::array<uint16_t, 7> capacity_by_day, std::vector<Student> students)
		: room_code(std::move(room_code)), capacity_by_day(std::move(capacity_by_day)), students(std::move(students)) {}

	Classroom(const Classroom& other) = default;
	Classroom(Classroom&& other) noexcept = default;
	Classroom& operator=(const Classroom& other) = default;
	Classroom& operator=(Classroom&& other) noexcept = default;
	~Classroom() = default;

	std::string room_code;
	std::array<uint16_t, 7> capacity_by_day;
	std::vector<Student> students;
};
#endif // STUDENT_H


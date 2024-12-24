CREATE TABLE IF NOT EXISTS LessonHours(
	lesson_num INTEGER PRIMARY KEY,
	start_time INTEGER NOT NULL,
	end_time INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS Lessons(
	week_day INTEGER NOT NULL,
	class_id INTEGER NOT NULL,
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	subject_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (class_id, lesson_hour)
);
CREATE TABLE IF NOT EXISTS Teachers(
	teacher_id INTEGER PRIMARY KEY,
	full_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS Classrooms(
	classroom_id INTEGER PRIMARY KEY,
	classroom_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS Subjects(
	subject_id INTEGER PRIMARY KEY,
	subject_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS Duties(
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (lesson_hour, teacher_id, classroom_id),
	FOREIGN KEY (teacher_id) REFERENCES Teachers(teacher_id),
	FOREIGN KEY (classroom_id) REFERENCES Classrooms(classroom_id),
	FOREIGN KEY (lesson_hour) REFERENCES LessonHours(lesson_num)
);CREATE TABLE IF NOT EXISTS lessons(
	class_id INTEGER NOT NULL,
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (class_id, lesson_hour)
);
CREATE TABLE IF NOT EXISTS lesson_hours(
	lesson_num INTEGER PRIMARY KEY,
	start_time INTEGER NOT NULL,
	end_time INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS teachers(
	teacher_id INTEGER PRIMARY KEY,
	full_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS classrooms(
	classroom_id INTEGER PRIMARY KEY,
	classroom_name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS duties(
	lesson_hour INTEGER NOT NULL,
	teacher_id INTEGER NOT NULL,
	classroom_id INTEGER NOT NULL,
	PRIMARY KEY (lesson_hour, teacher_id, classroom_id),
	FOREIGN KEY (teacher_id) REFERENCES teachers(teacher_id),
	FOREIGN KEY (classroom_id) REFERENCES classrooms(classroom_id),
	FOREIGN KEY (lesson_hour) REFERENCES lesson_hours(lesson_num)
);

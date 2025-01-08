use shared_components::types::*;
use std::collections::BTreeMap;
use shared_components::database;
use tokio::sync::MutexGuard;
use rusqlite::Connection;

fn admin_panel(){

}
//  subject,classroom,class
type LessonData = (String, String, String);
// Place
type DutyData = (u8, String);
type Hour = Orb<LessonData, Orb<DutyData, ()>>;

// Key: (week_day, lesson num), Value: (class, classroom, subject) 
type Lessons = BTreeMap<(u8, u8), (u16, u16, u16)>;

pub fn dashboard(name: &str, hour: Hour, lang: Language, lessons: Lessons, db: &MutexGuard<'_, Connection>) -> String{
    let mut max_lesson = 0;
    for (_, lesson_num) in lessons.keys(){
        if max_lesson < *lesson_num{
            max_lesson = *lesson_num;
        }
    }
    let (class_hashmap, classroom_hashmap, subject_hashmap) = 
        (
            database::get_classes(&db),
            database::get_classrooms(&db),
            database::get_subjects(&db),
        );
    let mut to_ret = "".to_string();
    for weekd in 1..=5{
        to_ret.push_str(&format!("<weekd><p>{}</p><lessons-wd>", crate::weekd_to_string(weekd)));
        for lesson_numb in 1..=max_lesson{
            to_ret.push_str("   <lesson>");
            if let Some((class, classroom, subject)) = lessons.get(&(weekd, lesson_numb)){
                to_ret.push_str(
                    &format!("<p><strong>{} {}</strong></p><p>{}</p><p>{}</p><p>{}</p>",
                        lang.english_or("Lesson", "Lekcja"),
                        lesson_numb,
                        subject_hashmap.get(subject).unwrap_or(&subject.to_string()),
                        class_hashmap.get(class).unwrap_or(&class.to_string()),
                        classroom_hashmap.get(classroom).unwrap_or(&class.to_string())
                    )
                );
            }
            else{
                to_ret.push_str("<p><strong>-</strong></p><p>-</p><p>-</p><p>-</p>");
            }
            to_ret.push_str("   </lesson>")
        }
        to_ret.push_str("</lessons-wd></weekd>");
    }
    format!
    (
    "
    <h1>{} {}!</h1>
    <h2>{}</h2>
    <lessons>
       {} 
    </lessons>
    ",
    lang.english_or("Welcome Back,", "Witaj spowrotem"), name,
    match hour{
        Orb::Data((subject, classroom, class)) => {
            lang.english_or(
                &format!("You have {} with {} in {}",
                    subject, class, classroom
                ),
                &format!("Masz teraz {} z {} w {}", 
                    subject, class, classroom
                )
            )
        },
        Orb::Alt(duty_orb) => {
            match duty_orb{
                Orb::Data((lesson_num, place)) => {
                    lang.english_or(
                        &format!("You have duty in {} after lesson {}!", place, lesson_num), 
                        &format!("Masz dyżur w {} po lekcji {}!", place, lesson_num)
                    )
                }
                Orb::Alt(_) => {
                    lang.english_or("You <strong>don't have</strong> Duty, enjoy break!", 
                        "<strong>Nie masz</strong> dyżuru, ciesz się przerwą!")
                }
            }
        }
    },
    to_ret
    )
}

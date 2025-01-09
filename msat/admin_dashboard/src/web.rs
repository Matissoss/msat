use shared_components::types::*;
use std::collections::BTreeMap;
use shared_components::database;
use tokio::sync::MutexGuard;
use rusqlite::Connection;

//  subject,classroom,class
type LessonData = (String, String, String);
// week day and class_id
type Current = Option<(u8, u8)>;
// Place
type DutyData = (u8, String);
type Hour = Orb<LessonData, Orb<DutyData, ()>>;

// Key: (week_day, lesson num), Value: (class, classroom, subject) 
type Lessons = BTreeMap<(u8, u8), (u16, u16, u16)>;

pub fn dashboard
(name: &str, current: Current, hour: Hour, lang: Language, lessons: Lessons, db: &MutexGuard<'_, Connection>) -> String{
    let mut max_lesson = 0;
    for (_, lesson_num) in lessons.keys(){
        if max_lesson < *lesson_num{
            max_lesson = *lesson_num;
        }
    }
    let (class_hashmap, classroom_hashmap, subject_hashmap, lesson_hours) = 
        (
            database::get_classes(&db),
            database::get_classrooms(&db),
            database::get_subjects(&db),
            database::get_lesson_hours(&db)
        );
    let mut to_ret = "".to_string();

    let currently = match current{
        Some((current_weekd, current_lessonn)) => (current_weekd, current_lessonn),
        None => (0, 0)
    };

    for weekd in 1..=5{
        to_ret.push_str(&format!("<weekd><p>{}</p><lessons-wd>", crate::weekd_to_string(&lang, weekd)));
        for lesson_numb in 1..=max_lesson{
            if currently == (weekd, lesson_numb){
                to_ret.push_str("<lesson class=\"current_lesson\">");
            }
            else{
                to_ret.push_str("<lesson>")
            }
            if let Some((class, classroom, subject)) = lessons.get(&(weekd, lesson_numb)){
                to_ret.push_str(
                    &format!("<p><strong>{} {}</strong></p><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
                        lang.english_or("Lesson", "Lekcja"),
                        lesson_numb,
                        lesson_hours.get(&lesson_numb).unwrap_or(&(0, 0)).msat_to_string(),
                        subject_hashmap.get(subject).unwrap_or(&subject.to_string()),
                        class_hashmap.get(class).unwrap_or(&class.to_string()),
                        classroom_hashmap.get(classroom).unwrap_or(&class.to_string())
                    )
                );
            }
            else{
                to_ret.push_str(&format!("<p><strong>.</strong><p>.</p></p><p>{}</p><p>.</p><p>.</p>", lang.english_or("No Data", "Brak danych")));
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
    <h3>{}</h3>
    <lessons>
       {} 
    </lessons>
    <h3>{}</h3>
    <duties>
    </duties>
    ",
    lang.english_or("Welcome Back,", "Witaj spowrotem"), name,
    match hour{
        Orb::Data((subject, classroom, class)) => {
            lang.english_or(
                &format!("You have <italic>{}</italic> with <italic>{}</italic> in <italic>{}</italic>",
                    subject, class, classroom
                ),
                &format!("Masz teraz <italic>{}</italic> z <italic>{}</italic> w <italic>{}</italic>", 
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
    lang.english_or("Lessons", "Lekcje"),
    to_ret,
    lang.english_or("Duties", "Dyżury")
    )
}

pub fn login(lang: Language) -> String{
    return format!(
        "
        <div class='login'>
        <h1>{}</h1>
        <col>
            <input id='id' type=number max=65536 min=1 placeholder='{}'>
            <input id='password' type=password placeholder='{}'>
            <button id='submit_but'>{}</button>
        </col>
        </div>",
        lang.english_or("Log in", "Zaloguj się"),
        lang.english_or("Enter your teacher ID", "Wstaw twój Identyfikator"),
        lang.english_or("Enter your password"  , "Wstaw hasło"),
        lang.english_or("Login", "Zaloguj się")
    );
}

pub fn post_login(lang: &Language) -> String{
    return format!(
        "
        <div class='login'>
            <h1>{}</h1>
            <h2>{}</h2>
        </div>
        ",
        lang.english_or("Login succesfully", "Zalogowano się pomyślnie"),
        lang.english_or("You can continue", "Możesz kontynuować")
    );
}

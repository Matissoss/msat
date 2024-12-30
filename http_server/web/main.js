function $(html){
	return document.getElementById(html);
}

let language = "en";

$("data").addEventListener('submit', async function (e) {
    e.preventDefault();

    const formData = new FormData(this);
    const params = new URLSearchParams();
    formData.forEach((value, key) => {
        params.append(key, value);
    });
    fetch('/?' + params.toString(), {
        method: 'GET' 
    })
    .then(response => response.text()) 
    .then(result => {
	    $("response").innerHTML = result;
	    $("input_args").value = "";
	    set_width();
    })
    .catch(() => {
	    $("response").innerHTML = "<db_col><db_row><p>Server didn't send any data. Check if your request is correct</p><p>Serwer nie wysłał żadnych informacji. Sprawdź czy twoje zapytanie jest poprawne.</p></db_row></db_col>";
	    set_width();
    });
})

function set_width(){
	    let width = 100 / $("response").children[0].children[0].childElementCount;
	    for (let i = 0; i < $("response").children[0].childElementCount; i++){
		    for(let j = 0; j < $("response").children[0].children[i].childElementCount; j++){
		    	$("response").children[0].children[i].children[j].style.width = `${width}vw`
		    }
	    }
}

$("input").addEventListener('change', (e) => {
	let data = e.target.value;
	if (data.includes("GET")){
		$("input_args").className = "non-visible"
		$("input_text").className = "non-visible";
		$("input_password").className = "non-visible";
	}
	else
	{
		$("input_text").className = "";
		$("input_args").className = "";
		$("input_password").className = "";
		if (data.includes("1")){
			if (language == "pl"){
			$("input_text").innerHTML = 
"(dzień tygodnia - cyfra (1-7)), (id nauczyciela - cyfra), (id klasy - cyfra), (id klasy(pomieszczenie) - cyfra), (id przedmiotu - cyfra), (numer lekcji - cyfra)"
			}
			else{
				$("input_text").innerHTML = "(week_day: number 1-7), (teacher id - number), (classroom_id - number), (subject id - number), (lesson number - number)"
			}
		}
		else if (data.includes("2")){
			if (language == "pl"){
				$("input_text").innerHTML = "(id nauczyciela - numer), (imię - tekst (jeden wyraz)), (nazwisko - tekst (jeden wyraz))";
			}
			else{
				$("input_text").innerHTML = "(teacher id - number), (first_name - text (one-word)), (last_name - text (one word))";
			}
		}
		else if (data.includes("3")){
			if (language == "pl"){
				$("input_text").innerHTML = "(numer lekcji - numer), (id nauczyciela - numer), (numer klasy (pomieszczenie) - numer), (numer tygodnia - numer 1-7)"
			}
			else{
				$("input_text").innerHTML = "(lesson number - number), (teacher id - number), (classroom id - number), (week_day - number 1-7)"
			}
		}
		else if (data.includes("4")){
			if (language == "pl"){
				$("input_text").innerHTML = "(id przedmiotu - numer), (nazwa przedmiotu - tekst (jedno słowo, zamiast spacji użyć: '_'))"
			}
			else{
				$("input_text").innerHTML = "(subject id - number), (subject name - text (one-word, instead of space use: '_'))"
			}
		}
		else if (data.includes("5")){
			if (language == "pl"){
				$("input_text").innerHTML = "(id klasy - numer), (nazwa klasy - tekst (jedno słowo, zamiast spacji użyć: '_'))"
			}
			else{
				$("input_text").innerHTML = "(class id - number), (class name - text (one-word, instead of space use: '_'))"
			}
		}
		else if (data.includes("6")){
			if (language == "pl"){
				$("input_text").innerHTML = "(id klasy (pomieszczenie) - numer), (nazwa klasy(pomieszczenie) - tekst (jedno słowo, zamiast spacji używać: '_'))"
			}
			else{
				$("input_text").innerHTML = "(classroom id - number), (classroom name - text (one-word, instead of space use: '_'))"
			}
		}
		else if (data.includes("7")){
			if (language == "pl"){
				$("input_text").innerHTML = "(numer lekcji - numer), (godzina rozpoczęcia - (np. 9:00, ale BEZ ':', czyli: 900)), (godzina zakończenia - tak jak godzina rozpoczęcia)"
			}
			else{
				$("input_text").innerHTML = "(lesson number - number), (start time: hhmm -> (fe. 9:00, but WITHOUT ':', so: 900)), (end time - same as start time)"
			}
		}
	}
});

function polish(){
	$("title").innerHTML = "Panel administratora msat";
	for (let t = 0; t<2; t++){
		let request_type = "";
		if (t == 0){
			request_type = "GET";
		}
		else{
			request_type = "POST";
		}
		for (let i=1;i<8;i++){
			let str = "";
			switch(i){
				case 1:
					if (request_type == "GET"){
						str = "Pobierz Dane o Lekcjach"
					}
					else{
						str = "Wstaw Dane o Lekcjach"
					}
					break;
				case 2:
					if (request_type == "GET"){
						str ="Pobierz Dane o Nauczycielach"
					}
					else{
						str = "Wstaw Dane o Nauczycielach"
					}
					break;
				case 3:
					if (request_type == "GET"){
						str ="Pobierz Dane o Dyżurach"
					}
					else{
						str = "Wstaw Dane o Dyżurach"
					}
					break;
				case 4:
					if (request_type == "GET"){
						str ="Pobierz Dane o Przedmiotach"
					}
					else{
						str = "Wstaw Dane o Przedmiotach"
					}
					break;
				case 5:
					if (request_type == "GET"){
						str ="Pobierz Dane o Klasach"
					}
					else{
						str = "Wstaw Dane o Klasach"
					}
					break;
				case 6:
					if (request_type == "GET"){
						str ="Pobierz Dane o Klasach (Pomieszczenie)"
					}
					else{
						str = "Wstaw Dane o Klasach (Pomieszczenie)"
					}
					break;
				case 7:
					if (request_type == "GET"){
						str ="Pobierz Dane o Godzinach Lekcyjnych"
					}
					else{
						str = "Wstaw Dane o Godzinach Lekcyjnych"
					}
					break;
			}
			$("input").innerHTML += `<option value="${request_type} ${i}">${str}</option>`;
		}
	}
	$("data-t").innerHTML = "Dane Wejściowe";
	$("lang-t").innerHTML = "Ustawienia";
	$("submit").innerHTML = "Wyślij zapytanie";
}
function english(){
	language = "en";
	$("title").innerHTML = "msat admin panel";
	for (let t = 0; t<2; t++){
		let request_type = "";
		if (t == 0){
			request_type = "GET";
		}
		else{
			request_type = "POST";
		}
		for (let i=1;i<8;i++){
			let str = "";
			switch(i){
				case 1:
					if (request_type == "GET"){
						str = "GET Lessons"
					}
					else{
						str = "POST Lessons"
					}
					break;
				case 2:
					if (request_type == "GET"){
						str ="GET Teachers"
					}
					else{
						str = "POST Teachers"
					}
					break;
				case 3:
					if (request_type == "GET"){
						str ="GET Duties"
					}
					else{
						str = "POST Duties"
					}
					break;
				case 4:
					if (request_type == "GET"){
						str ="GET Subjects"
					}
					else{
						str = "POST Subjects"
					}
					break;
				case 5:
					if (request_type == "GET"){
						str ="GET Classes"
					}
					else{
						str = "POST Classes"
					}
					break;
				case 6:
					if (request_type == "GET"){
						str ="GET Classrooms"
					}
					else{
						str = "POST Classrooms"
					}
					break;
				case 7:
					if (request_type == "GET"){
						str = "GET Lesson Hours"
					}
					else{
						str = "POST Lesson Hours"
					}
					break;
			}
			$("input").innerHTML += `<option value="${request_type} ${i}">${str}</option>`;
		}
	}
	$("data-t").innerHTML = "Input";
	$("lang-t").innerHTML = "Settings";
	$("submit").innerHTML = "Submit";
}

let query = window.location.search;
if (query.includes("?lang=pl")){
	language = "pl";
	polish();
}
else{
	english();
}

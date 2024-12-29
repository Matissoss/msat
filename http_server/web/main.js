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
	    let width = 100 / $("response").children[0].children[0].childElementCount;
	    for (let i = 0; i < $("response").children[0].childElementCount; i++){
		    for(let j = 0; j < $("response").children[0].children[i].childElementCount; j++){
		    	$("response").children[0].children[i].children[j].style.width = `${width}vw`
		    }
	    }
    })
    .catch(error => console.error('Błąd:', error));
})

$("input").addEventListener('change', (e) => {
	let data = e.target.value;
	if (data.includes("GET")){
		$("input_args").className = "non-visible"
	}
	else
	{
		if (data.includes("1")){
			$("input_args").placeholder = 
"(dzień tygodnia - cyfra (1-7)), (id nauczyciela - cyfra), (id klasy - cyfra), (id klasy(pomieszczenie) - numer), (id przedmiotu - cyfra), (numer lekcji - cyfra)"
		}
		$("input_args").className = ""
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
}

let query = window.location.search;
if (query.includes("?lang=pl")){
	language = "pl";
	polish();
}
else{
	english();
}

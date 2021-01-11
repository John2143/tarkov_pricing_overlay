let https = require("https");

let baseurl = "https://escapefromtarkov.gamepedia.com/api.php?action=query&format=json&list=allpages";

let apcontinue = "\"Big Stick\" 9x19 magazine for Glock 9x19";
//let apcontinue = ".338_Lapua_Magnum_FMJ";

function GET(url) {
    return new Promise((resolve, reject) => {
        let k = https.get(url, d => {

            let data = "";

            d.on("data", c => { data += c; });
            d.on("end", () => {
                try {
                    resolve(JSON.parse(data));
                } catch(e) {
                    reject(data);
                }
            });
        });
        k.on("error", e => { reject(e); });
    });
}

async function main() {
    console.log("ya");
    for(;;){
        let url = baseurl + "&apcontinue=" + encodeURIComponent(apcontinue) + "&*"

        let d = await GET(url);

        apcontinue = d.continue.apcontinue;

        let titles = d.query.allpages.map(x => x.title);
        console.log("!" + apcontinue);

        for(let t of titles) {
            console.log(t);
        }
    }
}

main()
    .catch(console.error)
    .then(console.log);

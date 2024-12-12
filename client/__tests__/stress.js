import fetch from 'node-fetch'
import { Engine } from '../static/engine.js'
import { Storage } from '../static/storage.js';



fetch("http://127.0.0.1:3000/get_info")
    .then((response) => response.json())
    .then((data) => {
    console.log(data);
    
    const tasks = [];
    const engines = []; 
    
    console.log = function() {};

    for (let i = 0; i < 300; i++) {
        engines.push(new Engine(new Storage(), false));
        
        tasks.push(
            new Promise((resolve, reject) => {
                engines[i].setup(data, () => {
                    engines[i].sendToServer(`hi ${i + 1}`);
                    resolve();
                });
            })
        );
    }

    Promise.all(tasks).then(() => {
        console.log("All tasks are complete.");
    });

    setTimeout(() => { // 15000ms = 15초
        process.exit();
      }, 300000);  
});


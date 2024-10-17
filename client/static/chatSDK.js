var msgerForm;
var msgerInput;
var msgerChat;
var portInput;
var portForm;
var socket;

window.onload = function(){
    msgerForm = get(".msger-inputarea");
    msgerInput = get(".msger-input");
    msgerChat = get(".msger-chat");
    portForm = get(".port-inputarea");
    portInput = get(".port-input");

    portForm.addEventListener("submit", event => {
      event.preventDefault();

      const port = portInput.value;
      if(isNaN(port)) return;

      const res = connectServer(port);

    })

    msgerForm.addEventListener("submit", event => {
        event.preventDefault();
      
        const msgText = msgerInput.value;
        if (!msgText) return;
      
        appendMessage(PERSON_NAME, PERSON_IMG, "right", msgText);
        msgerInput.value = "";

        const currentTimeInSeconds = Math.floor(Date.now() / 1000);
        // JavaScript 객체 생성
        const clientMsg = {
          uid: "highcloud",           // String으로 설정
          data: msgText,            // String으로 설정
          time: currentTimeInSeconds,  // Rust에서 UNIX 타임스탬프로 변환된 값
          cur_seq: 1,              // u64에 해당하는 정수
          timestamp: 1,
        };

        socket.send(JSON.stringify(clientMsg));
      
        // botResponse();
      });
}


const BOT_MSGS = [
  "Hi, how are you?",
  "Ohh... I can't understand what you trying to say. Sorry!",
  "I like to play games... But I don't know how to play!",
  "Sorry if my answers are not relevant. :))",
  "I feel sleepy! :("
];

// Icons made by Freepik from www.flaticon.com
const BOT_IMG = "https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Headshot__0087_cropped_leslie-lamport.jpg";
const PERSON_IMG = "https://www.gravatar.com/avatar/056effcac7fca237926f57ba2450429a";
const BOT_NAME = "Leslie Lamport";
const PERSON_NAME = "Diego Ongaro";

function connectServer(port){
  socket = new WebSocket("ws://localhost:"+port);
  console.log("connect to " + "localhost:"+port);

  socket.onopen = () => {
    console.log("WebSocket is open");
    portInput.style.backgroundColor =  'rgb(0, 196, 65)';
  };

  socket.onmessage = (event) => {
    console.log("Message from server:", event.data);
    botResponse(event.data);
  };

  socket.onerror = (error) => {
    console.log("Error has occured:", error);
    portInput.style.backgroundColor = 'red';
  };

}

function appendMessage(name, img, side, text) {
  //   Simple solution for small apps
  const msgHTML = `
    <div class="msg ${side}-msg">
      <div class="msg-img" style="background-image: url(${img})"></div>

      <div class="msg-bubble">
        <div class="msg-info">
          <div class="msg-info-name">${name}</div>
          <div class="msg-info-time">${formatDate(new Date())}</div>
        </div>

        <div class="msg-text">${text}</div>
      </div>
    </div>
  `;

  msgerChat.insertAdjacentHTML("beforeend", msgHTML);
  msgerChat.scrollTop += 500;
}

function botResponse(msgText) {
  // const r = random(0, BOT_MSGS.length - 1);
  // const msgText = BOT_MSGS[r];
  const delay = msgText.split(" ").length * 100;

  console.log("responsing");

  setTimeout(() => {
    appendMessage(BOT_NAME, BOT_IMG, "left", msgText);
  }, delay);
}

// Utils
function get(selector, root = document) {
  return root.querySelector(selector);
}

function formatDate(date) {
  const h = "0" + date.getHours();
  const m = "0" + date.getMinutes();

  return `${h.slice(-2)}:${m.slice(-2)}`;
}

function random(min, max) {
  return Math.floor(Math.random() * (max - min) + min);
}

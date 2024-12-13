/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
*/

import { MsgHandler } from "./message.js";
import * as utils from "./utils.js";
import { Storage } from "./storage.js";

export class Engine {
  id;
  userId;
  committedIndex;
  currentHost;
  currentPort;
  limitsOfRetryConnect = 10;
  numOfRetryConnect = 0;

  socket;
  msgHandler;

  // Icons made by Freepik from www.flaticon.com
  OTHER_IMG =
    "https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Headshot__0087_cropped_leslie-lamport.jpg";
  USER_IMG = "https://www.gravatar.com/avatar/056effcac7fca237926f57ba2450429a";

  // html attributes
  constructor(storage, enable_dom) {
    this.id = utils.getRandomString(7);
    this.userId = utils.getRandomString(7);
    this.committedIndex = 0; // committed index that client want to receive
    this.enable_dom = enable_dom;
    this.limitsOfTimeout = 5;
    this.retry_flag = 0;

    this.storage = storage;
    this.msgHandler = new MsgHandler(this.storage);

    this.pingSent = -1;
    this.pingLimitTime = 10000; //ms

    if(enable_dom == false){
      return;
    }
    this.msgerForm = utils.get(".msger-inputarea");
    this.msgerBtn = utils.get(".msger-send-btn");
    this.msgerInput = utils.get(".msger-input");
    this.msgerChat = utils.get(".msger-chat");
    this.portForm = utils.get(".port-inputarea");
    this.connectBtn = utils.get(".connect-btn");
    this.notCommittedChat = utils.get(".nc-msger-chat");
    this.serverInfoDiv = utils.get(".server-info");
    
    this.msgerBtn.disabled = true;

    this.portForm.addEventListener("submit", (event) => {
      event.preventDefault();
      this.connectBtn.disabled = true;

      fetch("/get_info")
        .then((response) => response.json())
        .then((data) => {
          console.log(data);
          this.setup(data);
        });
    });

    this.msgerForm.addEventListener("submit", this.sendToServerHandler.bind(this));
  }

  // initialize status from data and storage.
  setup(data, callback){
    this.info = data;

    // load from storage
    if (this.info.refresh_token == this.storage.getRefreshToken()) {
      try {
        this.committedIndex = parseInt(this.storage.getLatestIdx());
        this.id = this.storage.getId();
        this.userId = this.storage.getUid();

        console.log(
          this.committedIndex + " " + this.id + " " + this.userId
        );

        // print message on html
        if(this.enable_dom == true){
          for (let i = 0; i < this.committedIndex; i++) {
            let msg = this.storage.getMessage(i);
  
            if (msg.id == this.id) {
              this.appendCommittedMessage(
                msg.user_id,
                utils.genImage(msg.user_id),
                "right",
                msg.content,
                msg.time,
                i
              );
            } else {
              this.appendCommittedMessage(
                msg.user_id,
                utils.genImage(msg.user_id),
                "left",
                msg.content,
                msg.time,
                i
              );
            }
          }
        }
      } catch (err) {
        console.log("reading storage" + err);
      }
    } else {
      this.storage.
        reset(this.id, this.userId, this.info.version, this.info.refresh_token);
    }

    this.connectWS(
      this.info.domains[this.info.self_domain_idx],
      this.info.socket_ports[this.info.self_domain_idx],
      callback
    );
  }

  connectWS(host, port, callback) {
    this.currentHost = host;
    this.currentPort = port;

    try {
      this.socket = new WebSocket("ws://" + host + ":" + port);
    } catch (err) {
      console.log(err);
      return -1;
    }

    console.log("connect to " + host + " : " + port);

    this.interval_handler;

    this.socket.onopen = () => {
      console.log("WebSocket is open");
      this.retry_flag = 0;
      this.pingSent = -1;
      
      //update server info ui
      if(this.enable_dom == true){
        this.connectBtn.style.backgroundColor = "rgb(118, 255, 167)";
        this.serverInfoDiv.style.color = "rgb(0, 196, 65)";
        this.serverInfoDiv.innerHTML = "DEST > " + host + ":" + port;
        this.msgerBtn.disabled = false;
      }
      
      this.retransmission.bind(this);
      this.interval_handler = setInterval(this.retransmission.bind(this), 5000);

      if(callback) callback();
    };

    this.socket.onmessage = (event) => {
      this.numOfRetryConnect = 0;
      if(event.data == "pong"){
        //console.log("pong received");
        this.pingSent = Date.now();
      }
      else {
        console.log("Message from server:", event.data);
        this.updateState(event.data);
      }
    };

    this.socket.onerror = (error) => {
      // repeat connect the other server
      console.log("Error has occured:", error);

      if(this.enable_dom == true){
        this.connectBtn.style.backgroundColor = "red";
        this.serverInfoDiv.style.color = "red";
      }
      
      this.socket.close();
    };

    this.socket.onclose = (error) => {
      console.log("Connection is closed", error);
      
      if(this.enable_dom == true){
        this.connectBtn.style.backgroundColor = "red";
        this.serverInfoDiv.style.color = "red";
      }
      
      clearInterval(this.interval_handler);
      setTimeout(this.retry_connection.bind(this), 1000);
    };
    return 0;
  }

  retry_connection() {
    console.log("retry_connect");

    // block retry connect forever.
    if (this.numOfRetryConnect > this.limitsOfRetryConnect) return;
    this.numOfRetryConnect += 1;

    let new_host;
    let new_port;
    while (1) {
      let rand_idx = Math.floor(Math.random() * this.info.domains.length);
      new_host = this.info.domains[rand_idx];
      new_port = this.info.socket_ports[rand_idx];

      console.log(new_host + " " + new_port + ":" + this.currentHost);

      if (new_host != this.currentHost || new_port != this.currentPort) {
        break;
      }
    }
    this.connectWS(new_host, new_port);
  }

  retransmission() {
    // block retry connect forever.
    if (this.numOfRetryConnect > this.limitsOfRetryConnect) return;
    //console.log("retransmission");

    // if time outed -> reset msg index to 0, msg size to 1
    let numOfTimeout = this.msgHandler.timeoutCheck();
    
    let pongFail = false;

    // if ping time exceeds timeout, close immediately
    if(this.pingSent!=-1 && Date.now()-this.pingSent > this.pingLimitTime){
      console.log("pong time out" + Date.now()-this.pingSent + " > " + this.pingLimitTime + ":" + this.pingSent);
      pongFail = true
    }   
    // send ping
    this.socket.send("ping")

    // ping pong -> server dead ?
    // timeout -> server is so slow ...
    // -> reconnect other server
    if ( pongFail || numOfTimeout >= this.limitsOfTimeout) {
      console.log(numOfTimeout + " > " + this.limitsOfTimeout);
      console.log("close immediately");

      let prev_socket = this.socket;
      // force close
      if (prev_socket.onclose) {
        prev_socket.onclose({
          code: 1000,
          reason: "forced close",
          wasClean: true,
        });
      }

      // drop prev handler and close prev socket
      prev_socket.onclose = null;
      prev_socket.onerror = null;
      prev_socket.close();
      this.msgHandler.resetNumOfTimeOut();
      return;
    }

    if (this.socket.readyState == 1) {
      let msgArray = this.msgHandler.toJsonArray();
      let json = {
        committed_index: this.committedIndex,
        messages: msgArray,
      };

      if (msgArray.length != 0) {
        this.socket.send(JSON.stringify(json));
        console.log("retrans: " + JSON.stringify(json));
      }
    }
  }

  sendToServerHandler(event){
    event.preventDefault();
    this.sendToServer(this.msgerInput.value);

  }

  sendToServer(msgText) {
    if (!msgText) return;

    // 0. Save message into msgHandler.
    let timestamp = this.msgHandler.append(this.id, this.userId, msgText);

    // 1. Update html

    if(this.enable_dom==true){
      this.appendNotCommittedMessage(
        this.userId,
        utils.genImage(this.userId),
        "right",
        msgText,
        timestamp
      );
      this.msgerInput.value = "";
    }
    
    // 2. Send messages from msgHandler.
    let msgArray = this.msgHandler.toJsonArray();

    let json = {
      committed_index: this.committedIndex,
      messages: msgArray,
    };

    if (this.socket.readyState != 1) return;

    this.socket.send(JSON.stringify(json));
    console.log("send to server:" + JSON.stringify(json));
  }

  updateState(serverMsgs) {
    let serverMsg = JSON.parse(serverMsgs);
    let deletedFlag = false;

    // 1. Update committed index
    // this.committedIndex = serverMsg.committed_index+1;

    // 2. Update html
    for (let i = 0; i < serverMsg.length; i++) {
      let msg = serverMsg[i].message;
      let msgs_committed_idx = serverMsg[i].committed_index;

      if (msgs_committed_idx != this.committedIndex) {
        continue;
      }
      this.committedIndex++;

      // save in storage
      this.storage.saveMessage(msgs_committed_idx, msg);
      this.storage.setLatestIdx(this.committedIndex);

      if (msg.id == this.id) {
        // Clean up msgHandler
        this.msgHandler.cleanUp(msg.time_stamp);

        if(this.enable_dom == false){
          continue;
        }

        try {
          // Clean up not committed chat
          let parent = document.getElementById("nc-chat-area");
          let child = document.getElementById(msg.time_stamp);
          parent.removeChild(child);
        } catch (err) {
          console.log("there is no element to delete");
          console.log(err);
        }

        this.appendCommittedMessage(
          msg.user_id,
          utils.genImage(msg.user_id),
          "right",
          msg.content,
          msg.time,
          msgs_committed_idx
        );
        deletedFlag = true;
      } else {
        if(this.enable_dom == false){
          continue;
        }

        this.appendCommittedMessage(
          msg.user_id,
          utils.genImage(msg.user_id),
          "left",
          msg.content,
          msg.time,
          msgs_committed_idx
        );
      }
    }

    // 3. double msg size
    if (deletedFlag) {
      this.msgHandler.doubleMsgSize();
    }

    // // 4. ack committed idx
    // let json = {
    //     committed_index: this.committedIndex,
    //     messages: [],
    // }

    // this.socket.send(JSON.stringify(json));
    // console.log("send to server:" + JSON.stringify(json))
  }

  appendNotCommittedMessage(name, img, side, text, time_stamp) {
    const msgHTML = `
        <div class="msg ${side}-msg" id=${time_stamp}>
          <div class="msg-img" style="background-image: url(${img})"></div>
    
          <div class="nc-msg-bubble">
            <div class="msg-info">
              <div class="msg-info-name">${name}</div>
              <div class="msg-info-time">${utils.formatDate(new Date())}</div>
            </div>
    
            <div class="msg-text"><pre>${utils.insertLineBreaks(
              text
            )}</pre> </div>
          </div>
        </div>
      `;

    this.notCommittedChat.insertAdjacentHTML("beforeend", msgHTML);
    this.notCommittedChat.scrollTop += 500;
  }

  appendCommittedMessage(name, img, side, text, time, committed_index) {
    //   Simple solution for small apps
    const msgHTML = `
          <div class="msg ${side}-msg">
            <div class="msg-img" style="background-image: url(${img})"></div>
      
            <div class="msg-bubble">
              <div class="msg-info">
                <div class="msg-info-name">${name}</div>
                <div class="msg-info-time">${committed_index} / ${utils.formatDate(
      new Date(time)
    )}</div>
              </div>
      
              <div class="msg-text">${text}</div>
            </div>
          </div>
        `;

    this.msgerChat.insertAdjacentHTML("beforeend", msgHTML);
    this.msgerChat.scrollTop += 500;
  }
}

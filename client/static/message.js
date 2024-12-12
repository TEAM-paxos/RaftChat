/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
 */

// error case
// 1. server down
// -> ( reconnect the other server )
// 2. server still live but message was lost. this case occure when
//    pass the message to raft leader but the raft leader goes down.
// -> ( retransmission )

export class MsgHandler {
  #numOfTimeout = 0;
  #msgQue = [];
  #msgSendTime = [];
  #msgSent = [];
  #sendIndexToServer = 0;
  #msgSize = 1;
  #msgLimit = 8;
  #msgTimeOut = 10000; //ms = 10sec
  #timeStamp = 0;

  getTimeStamp() {
    return this.#timeStamp;
  }

  setTimeStamp(timeStamp) {
    this.#timeStamp = timeStamp;
  }

  get getQue() {
    return [...this.#msgQue];
  }

  append(id, user_id, content) {
    // sync the other taps...
    this.#timeStamp = parseInt(localStorage.getItem("timeStamp"));

    this.#timeStamp += 1;

    localStorage.setItem("timeStamp", this.#timeStamp);

    this.#msgQue.push(new Msg(id, user_id, content, this.#timeStamp));
    this.#msgSendTime.push(Date.now());
    this.#msgSent.push(true);
    return this.#timeStamp;
  }

  // Return the nums of timeout
  // When timeout, reset sendIndex 0 and msgsize = 1
  timeoutCheck() {
    if (
      this.#msgSent.length == 0 ||
      this.#msgSent[0] == false ||
      Date.now() - this.#msgSendTime[0] < this.#msgTimeOut
    )
      return this.#numOfTimeout;

    // time out : recovery mode
    this.#sendIndexToServer = 0;
    this.#msgSize = 2;

    for (let i = 0; i < this.#msgQue.length; i++) {
      this.#msgSent[i] = false;
    }

    this.#numOfTimeout += 1;
    return this.#numOfTimeout;
  }

  resetNumOfTimeOut() {
    this.#numOfTimeout = 0;
  }

  doubleMsgSize() {
    this.#msgSize *= 2;
    if (this.#msgSize > this.#msgLimit) {
      this.#msgSize = this.#msgLimit;
    }
    console.log("double: " + this.#msgSize);
  }

  toJsonArray() {
    if (this.#sendIndexToServer >= this.#msgQue.length) {
      return [];
    }

    console.log("sendIndexToServer: " + this.#sendIndexToServer);
    console.log("msgQueLength: " + this.#msgQue.length);
    console.log("msgSize: " + this.#msgSize);

    let temp = [];

    let j = 0;
    for (let i = 0; i < this.#msgSize; i++) {
      j = i + this.#sendIndexToServer;

      if (j >= this.#msgQue.length) {
        j--;
        break;
      }
      temp.push(this.#msgQue[j].toJson());
      this.#msgSendTime[j] = Date.now();
      this.#msgSent[j] = true;
    }

    this.#sendIndexToServer = j + 1;

    return temp;
  }

  queSize() {
    return this.#msgQue.length;
  }

  // cleanUp must works like pop front.
  cleanUp(timeStamp) {
    for (let i = 0; i < this.#msgQue.length; i++) {
      if (this.#msgQue[i].timeStamp === timeStamp) {
        this.#msgQue.splice(i, 1); // 해당 인덱스의 msg 삭제
        this.#msgSendTime.splice(i, 1); // 해당 인덱스의 age 삭제
        this.#msgSent.splice(i, 1);
        this.#sendIndexToServer -= 1; // sendIndexToServer 감소
        if (this.#sendIndexToServer < 0) {
          this.#sendIndexToServer = 0;
        }
      }
    }
    this.#numOfTimeout = 0;
  }
}

export class Msg {
  #id;
  #userId;
  #content;
  #time;
  #timeStamp;

  constructor(id, userId, content, timeStamp) {
    this.#id = id;
    this.#userId = userId;
    this.#content = content;
    this.#time = new Date().toISOString();
    this.#timeStamp = timeStamp;
  }

  // Getter
  get id() {
    return this.#id;
  }

  get userId() {
    return this.#userId;
  }

  get content() {
    return this.#content;
  }

  get time() {
    return this.#time;
  }

  get timeStamp() {
    return this.#timeStamp;
  }

  toJson() {
    return {
      id: this.#id,
      user_id: this.#userId,
      content: this.#content,
      time: this.#time,
      time_stamp: this.#timeStamp,
    };
  }

  isEqual(other) {
    if (!(other instanceof Msg)) return false; // 다른 타입일 경우 false
    return (
      this.id === other.id &&
      this.user_id === other.user_id &&
      this.content === other.content &&
      this.time === other.time &&
      this.timeStamp === other.timeStamp
    );
  }
}

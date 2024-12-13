/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
*/

class LocalStorageMock {
  constructor() {
    this.store = {};
  }

  clear() {
    this.store = {};
  }

  getItem(key) {
    return this.store[key] || null;
  }

  setItem(key, value) {
    this.store[key] = String(value);
  }

  removeItem(key) {
    delete this.store[key];
  }
}

export class Storage {
  constructor() {
    if (typeof localStorage === "undefined" || localStorage === null) {
      this.localStorage = new LocalStorageMock();
    } else {
      this.localStorage = localStorage;
    }
  }
  setServerVersion(version) {
    this.localStorage.setItem("serverVersion", version);
  }

  getServerVersion() {
    return this.localStorage.getItem("serverVersion");
  }

  setRefreshToken(token) {
    this.localStorage.setItem("refreshToken", token);
  }

  getRefreshToken() {
    return this.localStorage.getItem("refreshToken");
  }

  setTimeStamp(timeStamp) {
    return this.localStorage.setItem("timeStamp", timeStamp);
  }

  getTimeStamp(timeStamp) {
    return this.localStorage.getItem("timeStamp", timeStamp);
  }

  setIdUid(id, uid) {
    this.localStorage.setItem("id", id);
    this.localStorage.setItem("uid", uid);
  }

  getUid() {
    return this.localStorage.getItem("uid");
  }

  getId() {
    return this.localStorage.getItem("id");
  }

  setLatestIdx(idx) {
    this.localStorage.setItem("latestIdx", idx);
  }

  getLatestIdx() {
    return this.localStorage.getItem("latestIdx");
  }

  // idx, msg : json
  saveMessage(idx, msg) {
    this.localStorage.setItem(idx, JSON.stringify(msg));
  }

  // idx -> json
  getMessage(idx) {
    return JSON.parse(this.localStorage.getItem(idx));
  }

  clear() {
    this.localStorage.clear();
  }

  reset(id, uid, version, refresh_token){
    this.localStorage.clear();
    this.localStorage.setItem("id", id);
    this.localStorage.setItem("uid", uid);
    this.localStorage.setItem("serverVersion", version);
    this.localStorage.setItem("refreshToken", refresh_token);
    this.localStorage.setItem("latestIdx", 0);
    this.localStorage.setItem("timeStamp", 0);
  }
}

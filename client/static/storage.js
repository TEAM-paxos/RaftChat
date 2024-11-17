/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
*/

export class Storage {

    setServerVersion(version) {
        localStorage.setItem('serverVersion', version);
    }

    getServerVersion() {
        return localStorage.getItem('serverVersion');
    }

    setTimeStamp(timeStamp) {
        return localStorage.setItem('timeStamp', timeStamp);
    }

    getTimeStamp(timeStamp) {
        return localStorage.getItem('timeStamp', timeStamp);
    }

    setIdUid(id, uid) {
        localStorage.setItem('id', id);
        localStorage.setItem('uid', uid);
    }

    getUid() {
        return localStorage.getItem('uid');
    }

    getId() {
        return localStorage.getItem('id');
    }

    setLatestIdx(idx) {
        localStorage.setItem('latestIdx', idx);
    }

    getLatestIdx() {
        return localStorage.getItem('latestIdx');
    }

    // idx, msg : json
    saveMessage(idx, msg) {
        localStorage.setItem(idx, JSON.stringify(msg));
    }

    // idx -> json
    getMessage(idx) {
        return JSON.parse(localStorage.getItem(idx));
    }

    clear() {
        localStorage.clear();
    }
}
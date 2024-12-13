/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
 */

import { Engine } from './engine.js'
import { Storage } from './storage.js';

let engine
// initialize
window.onload = function () {
    engine = new Engine(new Storage(), true);
}
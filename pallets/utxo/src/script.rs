// Copyright (c) 2021 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): A. Altonen
use codec::{Decode, Encode};
use frame_support::dispatch::Vec;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// TODO what is the limit we want to set?
const SCRIPT_MAX_SIZE: u16 = 10_000;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
pub enum ScriptType {
    P2pkh = 1,
    OpCreate = 2,
    OpCall = 3,
}

impl Default for ScriptType {
    fn default() -> Self {
        return ScriptType::P2pkh;
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Default)]
pub struct ScriptPubKey {
    pub(crate) stype: ScriptType,
    pub(crate) script: Vec<u8>,
    pub(crate) data: Vec<u8>,
}

impl ScriptPubKey {
    /// Crete new ScriptPubKey which defaults to an empty script with type P2PKH
    pub fn new() -> Self {
        Self {
            stype: ScriptType::default(),
            script: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Set script and type for it
    pub fn set_script(&mut self, stype: ScriptType, script: &Vec<u8>) -> Result<(), &'static str> {
        if script.len() > SCRIPT_MAX_SIZE.into() {
            return Err("Input script is too big!");
        }

        self.stype = stype;
        self.script = script.clone();
        Ok(())
    }

    pub fn set_data(&mut self, data: &Vec<u8>) {
        self.data = data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::{assert_err, assert_ok};

    #[test]
    fn new_script() {
        let script = ScriptPubKey::new();
        assert_eq!(script.stype, ScriptType::P2pkh);
        assert_eq!(script.script.len(), 0);
        assert_eq!(script.data.len(), 0);
    }

    #[test]
    fn edit_script() {
        let mut script = ScriptPubKey::new();

        let asm = vec![0u8; 128];
        assert_ok!(script.set_script(ScriptType::P2pkh, &asm));

        let asm = vec![0u8; (SCRIPT_MAX_SIZE + 1).into()];
        assert_err!(
            script.set_script(ScriptType::P2pkh, &asm),
            "Input script is too big!"
        );
    }
}

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
// Author(s): A. Altonen, A. Sinitsyn

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use pallet_utxo_rpc_runtime_api::UtxoApi as UtxoRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait UtxoApi<BlockHash> {
    #[rpc(name = "utxo_send")]
    fn send(&self, at: Option<BlockHash>) -> Result<u32>;

    // What means Vec<(u64, Vec<u8>)> ? Have a look at utxo/rpc/runtime-api/src/lib.rs
    #[rpc(name = "tokens_list")]
    fn tokens_list(&self, at: Option<BlockHash>) -> Result<Vec<(H256, Vec<u8>)>>;

    #[rpc(name = "nft_read")]
    fn nft_read(
        &self,
        at: Option<BlockHash>,
        id: H256,
    ) -> Result<Option<(/* Data url */ Vec<u8>, /* Data hash */ [u8; 32])>>;
}

/// A struct that implements the [`UtxoApi`].
pub struct Utxo<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> Utxo<C, M> {
    /// Create new `Utxo` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

/// Error type of this RPC API.
pub enum Error {
    /// The transaction was not decodable.
    DecodeError = 1,
    /// The call to runtime failed.
    RuntimeError = 2,
    /// The access to Storage failed
    StorageError = 3,
}

impl<C, Block> UtxoApi<<Block as BlockT>::Hash> for Utxo<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static,
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C::Api: UtxoRuntimeApi<Block>,
{
    fn send(&self, at: Option<<Block as BlockT>::Hash>) -> Result<u32> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        api.send(&at).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError as i64),
            message: "Unable to query dispatch info.".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn tokens_list(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(H256, Vec<u8>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.tokens_list(&at);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::StorageError as i64),
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn nft_read(
        &self,
        at: Option<<Block as BlockT>::Hash>,
        id: H256,
    ) -> Result<Option<(/* Data url */ Vec<u8>, /* Data hash */ [u8; 32])>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.nft_read(&at, id);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::StorageError as i64),
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }
}

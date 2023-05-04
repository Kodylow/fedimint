window.SIDEBAR_ITEMS = {"attr":[["apply",""]],"constant":[["API_ENDPOINT_TIMEOUT","How long to wait before timing out client connections"]],"enum":[["CoreError",""],["ParseAmountError",""]],"fn":[["msats","Shorthand for [`Amount::from_msats`]"],["sats","Shorthand for [`Amount::from_sats`]"]],"macro":[["__api_endpoint","Example"],["_dyn_newtype_define_inner",""],["_dyn_newtype_define_with_instance_id_inner",""],["_dyn_newtype_impl_deref_mut",""],["async_trait_maybe_send","async trait that use MaybeSend"],["dyn_newtype_define","Define “dyn newtype” (a newtype over `dyn Trait`)"],["dyn_newtype_define_with_instance_id",""],["dyn_newtype_impl_dyn_clone_passhthrough","Implement `Clone` on a “dyn newtype”"],["dyn_newtype_impl_dyn_clone_passhthrough_with_instance_id",""],["impl_db_lookup",""],["impl_db_record","This is a helper macro that generates the implementations of `DatabaseRecord` necessary for reading/writing to the database and fetching by prefix."],["maybe_add_send","MaybeSync can not be used in `dyn $Trait + MaybeSend`"],["maybe_add_send_sync","See `maybe_add_send`"],["module_dyn_newtype_impl_encode_decode","Implement `Encodable` and `Decodable` for a “module dyn newtype”"],["module_plugin_trait_define","Define a “plugin” trait"],["newtype_impl_eq_passthrough_with_instance_id",""],["plugin_types_trait_impl_common","Implements the necessary traits for all associated types of a `FederationServer` module."],["push_db_key_items",""],["push_db_pair_items",""],["push_db_pair_items_no_serde",""]],"mod":[["admin_client",""],["api",""],["bitcoin_rpc",""],["cancellable",""],["config","Fedimint toplevel config"],["consensus","The actual implementation of consensus"],["core","Fedimint Core API (common) module interface"],["db","Provides interfaces for ACID-compliant data store backends"],["encoding","This module defines a binary encoding interface which is more suitable for consensus critical encoding than e.g. `bincode`. Over time all structs that need to be encoded to binary will be migrated to this interface."],["epoch",""],["fmt_utils",""],["hex",""],["macros",""],["module",""],["multiplexed","Implementation of multiplexed peer connections"],["net","Networking for mint-to-mint and client-to-mint communiccation"],["outcome",""],["query",""],["task",""],["tiered",""],["tiered_multi",""],["time",""],["timing",""],["transaction",""],["txoproof",""],["util",""]],"struct":[["Amount","Represents an amount of BTC inside the system. The base denomination is milli satoshi for now, this is also why the amount type from rust-bitcoin isn’t used instead."],["FedimintApiHandler",""],["FedimintServer","Main server for running Fedimint consensus and APIs"],["Feerate",""],["OutPoint","`OutPoint` represents a globally unique output in a transaction"],["PeerId",""],["Tiered",""],["TieredMulti","Represents notes of different denominations."],["TieredMultiZip",""],["TieredSummary",""],["TransactionId","A transaction id for peg-ins, peg-outs and reissuances"]],"trait":[["BitcoinHash","Trait which applies to hashes of all types."],["HasApiContext","Has the context necessary for serving API endpoints"],["NumPeers","for consensus-related calculations given the number of peers"],["ServerModule",""]]};
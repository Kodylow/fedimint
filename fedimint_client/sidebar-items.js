window.SIDEBAR_ITEMS = {"fn":[["box_up_state","Not sure why I couldn’t just directly call `Box::new` ins [`states_to_instanceless_dyn`], but this fixed it."],["client_decoders",""],["get_client_root_secret","Fetches the client secret from the database or generates a new one if none is present"],["states_add_instance",""],["states_to_instanceless_dyn",""]],"macro":[["sm_enum_variant_translation",""]],"mod":[["db","Database keys used by the client"],["module","Module client interface definitions"],["sm","Client state machine interfaces and executor implementation"],["transaction","Structs and interfaces to construct Fedimint transactions"]],"struct":[["Client",""],["ClientBuilder",""],["ClientInner",""],["ClientSecret","Secret input key material from which the [`DerivableSecret`] used by the client will be seeded"],["DynGlobalClientContext","Global state and functionality provided to all state machines running in the client"],["ModuleGlobalClientContext","Global state given to a specific client module and state. It is aware inside which module instance and operation it is used and to avoid module being aware of their instance id etc."],["OperationLogEntry",""],["TransactionUpdates","See [`Client::transaction_updates`]"]],"trait":[["IGlobalClientContext",""]],"type":[["InstancelessDynClientInput",""],["InstancelessDynClientOutput",""],["ModuleGlobalContextGen",""]]};
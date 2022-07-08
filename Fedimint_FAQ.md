#### Original Minimint Announcement
"I'm currently working on MiniMint, a prototypical implementation of the federated e-cash idea. But it is general enough to also eventually support more complex federated applications running on it like shop systems (unstoppable market places 🤯), smart custody and whatever we come up with. I hope to be able to make the prototype available later this year for people to play around with. If you know Rust, are into distributed systems and want to get a sneak peek: just let me know and I can add you to the repo." - esirion

#### The Benefits of BFT Consensus
"The benefit of traditional BFT consensus is that there can be no "block invalidation" or forks. That means you can make users threshold-encrypt transaction to the federation. The encrypted tx is included in block N and consensus rules dictate that every honest member has to contribute their decryption share in block N+1 and if enough members are honest (general assumption with federations) then the tx gets executed in block N+1 because there will be enough decryption shares. Since the block with the tx commitment can not be reversed after decryption there should be no MEV problem imo." - esirion

#### Who am I trusting in the federation?
"You always have to assume that the federation as a whole is honest (single members might not be though). Just like with BTC where you assume that there is no collusion of >50% of the miners, otherwise all kinds of things can go wrong." - esirion

#### Why can't the mint track money flows and hurt anonimity?


#### How does the Mint Interact with the Lightning Network?

"I'm going with the smart contract inside federation approach.
* Outgoing: HTLC-like construction
* Incoming: Thre recipient threshold encrypts a preimage and offers it for sale (for the invoice amount - fee). Decryption starts on payment. The payment goes through if the decrypted preimage fits the hash, otherwise the buyer gets his money back. A gateway that wants to accept an incoming HTLC can now buy the preimage." -esirion

The money-for-preimage exchange needs to be atomic, so until PTLCs we need threshold encryption so the federation can guarantee that.

#### Are you trusting the Mint to do the lightning payment? What if the federation just takes the ecash and doesn't pay or receive the lightning invoice?

"The assumption is that 2/3rds of the members are honest. That allows cooperation to decrypt the preimage. It can then be queried from every (honest) member. Afaik just publishing the preimage more or less publicly (all members know, so also potentially malicious ones) is good enough, staged timeouts should make sure that nobody can cheat on the path, even if the whole world learned the preimage (after the funds to be sent to the recipient were committed).
It's an interesting observation that if I add an online requirement for users I can get rid of threshold encryption. Need to think about that. Main drawback seems DoS risk against channel owners: anonymous users can lock up funds till timeout. With threshold encryption they learn after ~2 consensus rounds if the encrypted preimage was valid and if not can immediately withdraw their money.

(the problem being the user just disappearing and not telling the preimage after funds were locked)

The current threshold encryption scheme has the drawback of letting potentially malicious federation members know of the preimage. But that can't be avoided. At most it could be delayed imo (and the user would need to wait for their funds during that time, unclear if useful). That makes the preimage an even worse send receipt. But it was already broken with on-path adversaries learning it. More things PTLCs might fix.
(I got that idea from someone's comment at the end of my session in Zurich (just let users generate secret). At that time I still thought i needed to wait for PTLCs entirely. I'd love to give credit but I totally forgot who it was xD)" - elsirion

#### Is there anything special going on with the blind signing? Or is it just a Blind Schnorr Sig in a threshold setting?
"Yes indeed, it's pairing based. Most blind Schnorr schemes were broken by Wagner02 There is a fix from 2019 (finally published 21 I think)  but it makes the protocol more complex. And building this fix into threshold protocols seemed very hard. So I went with a BLS based scheme similar to Vo et al.'s "A new threshold blind signature scheme from pairings"  scheme (which I mostly reinvented except for doing blinding additively instead of multiplicatively, only found the paper later).

(the work wasn't all for nothing, I have nicer proofs, but wasn't worth publishing so far, thinking about making it an appendix to the federated e-cash paper I'm writing)" - elsirion

#### Federated E-cash Bitcoin Explained with Ruben Somsen
https://www.youtube.com/watch?v=alyYNIX0m3o

#### Is there a special protocol for connecting between mints besides lightning? 
"For example, A@mintX wants to pay B@mintY, there is a peer C who trusts both mints, so the payment can go through" - fiatjaf

"That sounds totally possible if C is a LN  gateway that trusts both federations and A and B are users transacting. You might want that C can also be a user, but imo there's a reliability trade-off (if they go offline the moment you try to send via them your funds stay locked, potentially for a long time).
But when you are writing about multi-hop payments it each hop would need a GW as intermediary to bridge trust between federations.
If federations already trusted each other 100% (unlikely but necessary to accept each other's IOUs without discount) then they should rather become one big federation.
The GWs can price the risk through their fees and can only lose their own capital, not the backing funds of the federation.
Not sure if your proposal is that enticing in that case, it would be LN in worse imo. For you the appeal probably is that we could do very small HTLCs that can't be enforced on BTC?" - elsirion

"I think the question of risk/reward can be left to the market. You can route any way you like, and there will be different prices. The benefit of routing without BTC is that you won't incur any on-chain fees." - rubensomsen

"Oh, that's an interesting point, pure mint-to-mint GWs don't need LN at all in theory 😲. That would make it viable for people to set up "LN nodes" even if priced out of BTC very hypothetical rn imo, but interesting possibility.
Pure mint-to-mint can probably be optimized. Maybe even made non-interactive for the GW (just lock up funds in smart contracts in both federations and then users can send via these smart contract pairs somehow. The GW still absorbs risk and collects fees, but doesn't need to be online." - elsirion

##### Cross Mint Thought Experiment
"Setup A -> Mint1 -> Mint2 -> B. GW has smart contracts on 1 and 2 (SC1/SC2). A sends amount x to SC1, gets a signature from the federation attesting receipt. SC2 now says, you can claim 0.99*x in Mint2 tokens if you present a signature from Mint1 that you deposited amount x. A does so to pay B.
All the risk is absorbed by the GW, not the mints.
The smart contracts just say at which risk-discounted rates they accept proof of payments from other mints." - elsirion

"I see what you mean, the signature sort of acts as the hash pre-image, will have to think about that" - rubensomsen

"Yeah, kind of. Imo it's a whole different paradigm, more akin to atomic swaps. but yeah, so are HTLCs. newver saw them that way really." - elsirion

#### What sort of interaction is required between the federation and the lightning gateway?
"Like in theory the GW functionality might be split up/just run on the federations. Eventually I'd like to play with pay-for-compute using WASM inside federations. There could be a special-purpose standard library that allows to sign, blind sign, etc. basically everything that can be implemented as a threshold algorithm. Every member executes the same code and should get the same result." - elsirion

#### What Signature Schemes did you consider for blinding? ECDSA, Schnorr, BLS?
"Actually only really BLS based since that was easiest to transform into a threshold blind signature scheme. Had I found a working TBS scheme based on Schnorr with a proper security proof I'd have taken that too though. Schnorr might be possible, but probably much harder given how finicky even blind sigs alone are." -elsirion

#### What are the Consensus round steps for signing by the Federation Members?
"Currently there are ~3 steps:

1. submit transaction, proposed by member
2. consensus on tx, produce and broadcast blind sig shares
3. combine blind signature

and at step 2 you can already accept the payment, the actual issuance of the token is a mere formality if the federation isn't compromised in-between.
Naive version with KVACs otoh would be:

1. submit transaction, proposed by member
2. consensus on tx, generate and broadcast verification shares
3. verification using verification shares, produce an broadcast blind sig shares
4. combine blind signatures

Under the assumption that less than f+1 members want to soft-censor you (without breaking consensus rules) and the tx gets broadcasted to every member, you could pipeline it this way:

1. submit transaction to all members, members broadcast verification shares, all members propose tx
2. consensus on tx, verification using broadcasted verification shares, produce an broadcast blind sig shares
3. combine blind signatures

But you'd still need the fallback to the 4 step process since there is no consensus over the "mempool"." - elsirion

#### Is there a risk that federation members' identities will be doxxed? If it's criminalized, will there ever be enough valunteers to run federations?

"Regulating e-cash federations will be highly unlikely to happen and if it did, it would be even more difficult to enforce.

It's highly unlikely to be regulated because this would require changing regulation to include non commercial financial service provision which would seemingly cover a lot of other activity (eg. holding money for an older relative, buying something at a shop on behalf of a friend, maintaining a piggy bank for a child, asking a friend to temporarily hold some cash on your behalf, etc). This would be nonsensical and non scalable due to the shear volume which is why all regulations I've seen tend to have a "by way of business" clause to expressly exclude non commercial activity.

If regulators chose to make this non sensical and non scalable change, it would be very difficult to enforce. Like a ban on speeding, pirated video streaming, or throwing litter (trash) on the floor, small transgressions happen all the time and are tolerated because they can't realistically be enforced without a massive increase in the size and cost of the government's surveillance capability, likely dwarfing the potential increased tax and enforcement revenue. Major transgressions however can be more easily identified. A small Fedimint community wallet is analogous to a small transgression while running a significant Bitcoin exchange is more akin to a major transgression.

Therefore, I believe the logical result of perverse regulation would not be too prevent Fedimint groups from existing, but rather, it would disincentivise Fedimint groups from becoming too big or from accepting untrusted non-2nd party participants, which would only help their decentralisation.

Separately, there are no "owners" of federations, just community members and what I like to call community "guardians". I liken this to a tribe where the strongest and most reliable are chosen to guard the tribe. Just because they are guarding the tribe doesn't mean they own the tribe, it is more a task that falls to them due to their characteristics and capabilities." - obi

#### Why isn't fedimint more widely discussed as a custody alternative and scaling solution?
"people usually wait until a new technology is in production before really spreading the word about it, since there is a long history of vaporware in this industry" - clarkmoody

"The Fedimint protocol and the minimint reference implementation have great potential as a pillar of the Bitcoin open source ecosystem:

Bitcoin Core is of course the central pillar and is focused on censorship resistant store of value and money.

Lightning is the 2nd pillar focused on censorship resistant payments.

In time, Fedimint aims to the 3rd pillar by providing censorship resistant privacy preserving Bitcoin custody.

For nearly a year, I've really wanted to have an implementation of Fedimint available to start using. However, at this stage, I've come to realise that what is most required are passionate and capable Bitcoin open source developers to get involved and help turn the protocol into a functioning implementation. Having too many non (or former) engineers like me involved now would be premature. That's why the Federated Chaumian Mint and Future of Bitcoin Privacy talks are on the Open source stage at Bitcoin 2022. 

However, when Minimint gets to a certain level of code completion, we'll need a few people (like myself) to dog food the implementation and that's when the clarion call can be made. 🙂

Just like Lightning and even Bitcoin Core implementations took a while before they were able to safely process significant amounts, the same will be true for Fedimint." - obi

#### Would federations eventually be considerer regulated entities? They're commercial, they're changing money, they seem like targets for gov regulators to go after?

"Definitely yes, it would be considered a regulated activity if you charged a fee. Because that would be "by way of business". Still, this would be hard if not impossible regulation to enforce but it would clearly fall within regulation at that stage.

This will also act to incentivise groups to stay amongst trusted 2nd parties because people are already comfortable with helping family and friends with financial matters for free.

For example, have you ever bought Bitcoin or held Bitcoin on behalf of a friend or family member? 🙂" - obi

"One of the design choices was to not have the LN node be part of the federation for now and instead let any 3rd party gateway that trusts the federation route payments for users. That way they can be a commercial entity charging fees for a service that is time consuming indeed, managing a LN node, while the federation stays noncommercial." - elsirion

#### Will the 3rd party that runs the lightning gateway be able to get all the data about mint members trying to pay/receive outside invoices?
"Not necessarily. At first: yes. Next step: remove unnecessary meta data (only give destination, hash, amount). Maybe we can even go the hosted channel route and let the client do the routing to hide the recipient. But ideally blinded routes would do that eventually anyway." - esirion

#### How does the federation control against IOU inflation? If I lose my ecash tokes and want to pressure the federation members I know to print me more tokens, could they?
"Roughly, the exact recovery schemes are still being worked on. But generally there is on-chain bitcoin for every spendable ecash token, so the federation is always fully backed. So imo there is no question of "controlling inflation". If that becomes a problem the trust assumption of t on n members being honest was broken. Even a recovery process will involve burning the old tokens, the federation members would merely act as oracles attesting to the fact that you lost your keys or died. With that attestation a previously backed up key could be revealed that lets the user's client run the recovery automatically." - elsirion

"Interoperability with Lightning means you can move funds in and out of the mint instantly if you have concerns about fractional reserve" - clarkmoody

"Fully automated bank runs 😎" - elsirion

##### So the idea is people will be able to transfer out immediately so it'll be hard to maintain a fractional reserve?
"Precisely. The origin of fractional reserve was the realization that the warehouse recipes (notes) circulated instead of gold and most of the gold didn't need to be redeemed at once. Increasing the velocity / decreasing the cost of withdrawal keeps the mint honest." - clarkmoody

"transferring out would continue to require the majority of federation guardians to coordinate to allow these transactions.

As @elsirion mentioned earlier, it would also require a majority of federation guardians to perform any action that would inflate the money supply in the first place. This not occurring is a core assumption of a well functioning federation, hence why strong trust in the federation guardians is key." - obi

#### What incentivizes lightning nodes to perform payments on behalf of the federation?
"Fees: pay them more than they have to spend to pay the invoice. Pretty much like any LN node. There's a HTLC-like smart contract in the federation that can only be claimed by the gateway with the preimage (or by the user after a timeout). You fund the contract with e-cash and get e-cash out, but the contract itself doesn't have the same privacy properties, but since the sender is still anonymous that's OK imo." -elsirion

#### What is the expected tps for a federation?
"For decent latency with the current, not fully optimized code not more than a handful (see https://twitter.com/EricSirion/status/1521211682227474434). It's not super high priority to support high tps since the ideal deployment scenario is many small federations. But I'm pretty sure that given a slightly better BFT algo and some pipelining this number could be 10x-ed on very strong hardware. In that hypothetical scenario: if you really want 50tps you better dump 10k$ on each of the servers, but at that point you are an enterprise anyway and can do so.

(the benchmark ran 4 federation members on one 32C/64T server, so these numbers should translate to rather common 8C/16T machines, latencies are also not as dependent on the core count. Max frequency is a bit more important since a few of the hot paths aren't well parallelized. Core count merely determines at which point a evergrowing backlog develops, but latencies become unacceptable before that.)" - elsirion

#### How do backups work for ecash tokens? Don’t users of fedimint have to somehow keep the keys for the blinded tokens they receive? These keys work similarly to bitcoin private keys right? If they’re lost, funds are lost? And if they’re stolen funds can be stolen?

"You are right but the backing up of these tokens can be simpler. High level, one way that it can be useful to think of Fedimint is as mini banks(exchanges) for friends/family run by friends and family.

Some options we're considering:

a) Back up of tokens in the cloud(s) - this can simply resolve the occurrences of user loss, and theft online as a hacker or the cloud storage provider(s) themselves will not be able to spend the tokens without knowledge of and membership to the federation that the tokens belong to as well. This is distinct from private keys which once stolen can be used anywhere. This is similar to (but superior to) backing up the password to your bank account online without the details of the bank it relates to. The password alone without knowledge of the bank it relates to.

b) Sharded back up of tokens between the guardians' Fedimint servers - You can potentially treat the guardians' Fedimint servers as a decentralised cloud back up only available to the federation users. This way, in the event of loss, the user will need to contact the guardians and they can, after verification, send their shards back to the user's mobile app for local reconstitution. Given the trust model of federations, this does not convey more additional trust to the federation than has already been conveyed.

c) Some combination of the above plus optional 2nd factors like biometrics and potentially optional spending limits (time, velocity, amount).

d) Furthermore, users can join multiple federations and split their assets between them to decentralise the risk.

Initially (for MVP), we will not have a back up option so loss of one's phone will equal the loss of bitcoins. But this is because we want to focus on making sure the core federation logic and chaumian mint logic and lightning gateway integration logic are working well and we have a simple mobile app that can interface with a federation." - obi

##### Those are cool solutions but don’t they nullify the privacy benefits? ie if you have some sort of an account system that holds the keys for users and verifies them, can’t the custodians censor based on that knowledge?

"Normal keys once known are usable anywhere. tokens are only usable within the federation. In this sense, they are more like digital coupons for a shop.

As such, cloud back up and theft (the implicit risk of doing this) is a different risk as the thief would still need to locate and attend (aka. join) the shop (aka. Fedimint federation) the coupons are spendable in.

This is not the case for keys stored in the cloud.

Sharded back up is also different because with 1st party custody, you are effectively reducing your security to sharded second (or 3rd) party custody by doing this. 

Federations are federated second party custody from the outset so adding this feature doesn't change the risk.

The account system question is something we are discussing/considering. My personal view is that we give choice. Also, to determine users tokens would require collusion between guardians.

A minority of guardians would not be able to collude and use their shards to determine the balance of a given user." - obi

##### don’t the tokens allow deposit/withdrawal to mainnet as well? so in that sense their theft does mean theft of the balance?

"But only the federation can process the transaction. A hacker of your online cloud storage account would need to join the federation. This would require somehow finding the joining credentials of the federation (likely doxxing themselves) and there is no need to have these (federation) credentials be backed up by the user online.

Again. It's a choice. Federations don't remove the need for back ups per se. But we believe that we can take advantage of the unique trusted second party structure of federations to make back up and recovery simpler and more automated without implicitly changing the security model.

This (the implicit change if the security model) is something that is a common occurrence with 1st party hardware wallet seed back ups. Users effectively end up adopting a back up solution that converts their first party custody to second or third party and often hold their back up with a single second or third party who requires no collusion to spend their assets from anywhere in the world with no restrictions.

True first party back up of hardware wallet seed phrases would involve telling no one and using no second or third parties. Besides being very risky, it will make estate planning and inheritance execution extremely difficult or impossible.

In my experience, best practice 1st party seed back up protocols are often actually sharded second or third party back up and could therefore have similar privacy risks to the approaches mentioned above (although with your keys, your complete transaction history can also be determined which is not necessarily the case for tokens)." - obi

#### How about using a sharded cloud backup?
"The shard restore procedure fits the federated trust model very well because you trust the guardians anyway (and assume in the community banking use case that there is some relationship so they can e.g. verify your identity irl).

How it will work imo: by generating the e-cash nonces and blinding keys deterministically as proposed by @RubenSomsen you can restore your e-cash from a seed by either doxing your e-cash tokens to the federation OR by downloading a huge public transaction log from the federation to do so locally preserving privacy OR using bloom filters as a middle ground (add some noise by requesting more data than you need). Once you have that you can threshold-encrypt the seed to the federation and add a note that they should decrypt e.g. in case of loss of funds after ID verification.

Only if t-of-n federation members can be convinced that the stated condition is met they can work together to decrypt the seed to you (they don't learn it themselves, you give them a new key to which they encrypt their decryption shares, with t of which you can restore your seed)

That sounds complicated but can be fully automated except for the part of convincing the federation members the condition is met.

If you add one more step you can also construct a cool inheritance scheme:

1. Decrypt your will to the federation, adding a note to open when you are dead (so nobody knows who gets the money before you die).
2. Encrypt the seed to the federation, add a note that says to decrypt to whoever is mentioned in 1." - elsirion


#### Feedback from the Oslo Freedom Forum on Fedimint Pain Points and Usecases
"At Oslo Freedom Forum and there are many many people interested in using Fedimint from many different locations. I don't think we'll have an issue finding users or trial runners!🙏🏾

The main requirement is being able to safely, securely, simply, and (implicitly) secretly, custody Bitcoin. 

Main requirements:

- How to make it really really simple. Some people don't understand what Bitcoin is (even after lots of explanation) and kept thinking about it in terms of an online bank account.

Some were concerned about the cost of buying hardware wallets to secure relatively small amounts.

Most were concerned about freedom from oppressive authoritarian regimes as it is common for freedom fighters to have their banking shut down or restricted.

From my observation, all really need financial and information privacy, but most don't think about or voice this requirement consciously.

Low trust in government institutions and high trust in friends and family is the norm and the only way they survive.

There is a really strong need (it's almost essential) for a simple way to convert to and from BTC to local fiat as well as many can't get their heads around the volatility and they are focused on other things (like not being assassinated, fighting for their people, escaping violent regimes, reporting in very dangerous environments, etc, etc.)" - obi

#### Would anyone be able to run a lightning gateway node, even if they're not in the federation?
"They would need to manage liquidity both on LN and keep a matching stash of mint tokens. They are essentially just a money-changer. They trust federation but federation doesn't trust them. Particular federations may choose to restrict who can participate as gateway but that's their decision. Once FROST happens the federation itself could run multisig / federated lightning node itself and you wouldn't need a gateway anymore, or so @elsirion tells me 😅" - justinmoon

#### Can a federation be given a blacklist of which e-cash tokens are "tainted" or should not be redeemable?
"In the current implementation the federation has no idea who owns what. The federation owns UTXOs and the users own blinded IOUs to those UTXOs. A judge couldn't know which blinded IOUs are owned by which users so they couldn't produce a blacklist ..." - justinmoon

#### Why not just help everyone self custody their UTXOs, run their  own lightning nodes? Why bother with a federated custodial option at all?
"I think there will be people eventually that never go on-chain and only use the LN bridges because they don't have the capital to justify creating a UTXO. We aren't there yet of course.

The more convincing argument currently is imo that many don't want to run any services and the self-sovereign model tends to require you to run services that are online from time to time and have certain rough edges. Together that leads to worse UX -> people don't use it. I believe federations can provide superior UX for reasonable engineering effort.

If you accept that there will be be people who choose to have someone/something custody their coins, then fedimint is a strict improvement for that set of users. Localize trust and add some privacy. The government attack problem exists just as much for centralized LSPs and becomes much harder the more distributed the custodians are." - elsirion

#### Would anyone be able to run a lightning gateway who also had an account with the mint? I think they would need to manage liquidity both on LN and keep a matching stash of mint tokens though?
"In theory (and in future), if a federation is connected to multiple lightning nodes, there is a possibility that LSPs can use the federations they are connected to to rebalance their channels off chain as opposed to using something like Lightning Loop." - obi

#### Given the LSP to federation trust dynamic, would it theoretically be possible to have a future version of Fedimint federation operate in "cold storage mode" where the keys for on chain transactions are kept offline with regular manually signed batch transactions by guardians, including to LSPs with a separate system for processing intra Fedimint token transactions?

"That's certainly possible. It wouldn't give you all that much protection though since in case of a security breach on t of n members (e.g. 3 of 4) the adversary could issue fake e-cash tokens instead. The only way to correct it if noticed in time would be requiring everyone to unblind their tokens, doxing some of their past transactions, and revert to a state before the breach." - elsirion
// import {SubstrateExtrinsic,SubstrateEvent,SubstrateBlock} from "@subql/types";
import {SubstrateEvent} from "@subql/types";
import {Transfer} from "../types";
import {Balance} from "@polkadot/types/interfaces";

// Here is where all the coding/transformations will take place.

/*
export async function handleBlock(block: SubstrateBlock): Promise<void> {
    //Create a new starterEntity with ID using block hash
    let record = new Transfer(block.block.header.hash.toString());
    //Record block number
    record.field1 = block.block.header.number.toNumber();
    await record.save();
}
*/

// this demo focuses on the events handler, so the other two can be commented out
export async function handleEvent(event: SubstrateEvent): Promise<void> {
    const {event: {data: [account, balance]}} = event;
    //Retrieve the record by its ID
    const record = await Transfer.get(event.extrinsic.block.block.header.hash.toString());
    record.field2 = account.toString();
    //Big integer type Balance of a transfer event
    record.field3 = (balance as Balance).toBigInt();
    await record.save();
}

/*
export async function handleCall(extrinsic: SubstrateExtrinsic): Promise<void> {
    const record = await Transfer.get(extrinsic.block.block.header.hash.toString());
    //Date type timestamp
    record.field4 = extrinsic.block.timestamp;
    //Boolean tyep
    record.field5 = true;
    await record.save();
}
*/


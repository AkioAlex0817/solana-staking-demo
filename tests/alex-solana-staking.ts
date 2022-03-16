import * as anchor from "@project-serum/anchor";
import {Program} from "@project-serum/anchor";
import utils from "./utils";
import {AlexSolanaStaking} from "../target/types/alex_solana_staking";
import * as fs from "fs";
import {Token, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import * as assert from "assert";
import {exit} from "process";

let program = anchor.workspace.AlexSolanaStaking;

const envProvider = anchor.Provider.env();
let provider = envProvider;

function setProvider(p) {
    provider = p;
    anchor.setProvider(p);
    program = new anchor.Program(program.idl, program.programId, p);
}

setProvider(provider);
describe("alex-solana-staking", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.Provider.env());

    let xTokenMintKey: anchor.web3.Keypair;
    let xTokenMintObject: Token;
    let xTokenMintPubkey: anchor.web3.PublicKey;
    let yTokenMintKey: anchor.web3.Keypair;
    let yTokenMintObject: Token;
    let yTokenMintPubkey: anchor.web3.PublicKey;

    let poolPubkey: anchor.web3.PublicKey;
    let poolBump: number;

    const program = anchor.workspace.AlexSolanaStaking as Program<AlexSolanaStaking>;

    it("Is initialized!", async () => {
        program.addEventListener('LogHandler', (e, s) => {
            console.log("Amount: ", e.amount.toString());
        });
        let rawData = fs.readFileSync('/home/alex/blockchain/solana-staking-demo/tests/keys/fromToken.json', "utf-8");
        let keyData = JSON.parse(rawData);
        xTokenMintKey = anchor.web3.Keypair.fromSecretKey(new Uint8Array(keyData));
        //xTokenMintKey = anchor.web3.Keypair.generate();
        xTokenMintObject = await utils.createMint(xTokenMintKey, provider, provider.wallet.publicKey, null, 0, TOKEN_PROGRAM_ID);
        xTokenMintPubkey = xTokenMintObject.publicKey;
        console.log("XTokenPubKey: ", xTokenMintPubkey.toString());
        [poolPubkey, poolBump] = await anchor.web3.PublicKey.findProgramAddress([xTokenMintPubkey.toBuffer()], program.programId);
        rawData = fs.readFileSync('/home/alex/blockchain/solana-staking-demo/tests/keys/toToken.json', "utf-8");
        keyData = JSON.parse(rawData);
        yTokenMintKey = anchor.web3.Keypair.fromSecretKey(new Uint8Array(keyData));
        //yTokenMintKey = anchor.web3.Keypair.generate();
        yTokenMintObject = await utils.createMint(yTokenMintKey, provider, poolPubkey, null, 0, TOKEN_PROGRAM_ID);
        yTokenMintPubkey = yTokenMintObject.publicKey;
        console.log("YTokenPubKey: ", yTokenMintPubkey.toString());
        [poolPubkey, poolBump] = await anchor.web3.PublicKey.findProgramAddress([xTokenMintPubkey.toBuffer()], program.programId);
        console.log("Pool PubKey: ", poolPubkey.toString());
        await program.rpc.initialize(
            poolBump,
            {
                accounts: {
                    xTokenMint: xTokenMintPubkey,
                    tokenPool: poolPubkey,
                    initializer: provider.wallet.publicKey,
                    systemProgram: anchor.web3.SystemProgram.programId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                }
            }
        );
    });

    let walletXTokenAccount: anchor.web3.PublicKey;
    let walletYTokenAccount: anchor.web3.PublicKey;
    it('Mint 100 X-Token', async () => {
        walletXTokenAccount = await xTokenMintObject.createAssociatedTokenAccount(provider.wallet.publicKey);
        walletYTokenAccount = await yTokenMintObject.createAssociatedTokenAccount(provider.wallet.publicKey);
        await utils.mintToAccount(provider, xTokenMintPubkey, walletXTokenAccount, 100);
        assert.strictEqual(await utils.getTokenBalance(provider, walletXTokenAccount), 100);
    });

    it('Start Staking: X-Token send to pool', async () => {
        await program.rpc.stake(
            poolBump,
            new anchor.BN(5),
            {
                accounts: {
                    xTokenMint: xTokenMintPubkey,
                    yTokenMint: yTokenMintPubkey,
                    sender: walletXTokenAccount,
                    senderAuthority: provider.wallet.publicKey,
                    tokenPool: poolPubkey,
                    receiver: walletYTokenAccount,
                    tokenProgram: TOKEN_PROGRAM_ID,
                }
            }
        );
        assert.strictEqual(await utils.getTokenBalance(provider, walletXTokenAccount), 95);
        assert.strictEqual(await utils.getTokenBalance(provider, walletYTokenAccount), 5);
        assert.strictEqual(await utils.getTokenBalance(provider, poolPubkey), 5);
    });

    it('Airdrop some tokens to the pool', async () => {
        await utils.mintToAccount(provider, xTokenMintPubkey, poolPubkey, 1);

        assert.strictEqual(await utils.getTokenBalance(provider, walletXTokenAccount), 95);dlwp
        assert.strictEqual(await utils.getTokenBalance(provider, walletYTokenAccount), 5);
        assert.strictEqual(await utils.getTokenBalance(provider, poolPubkey), 6);
    });

    it('Redeem Y-Token for X-Token', async () => {
        await program.rpc.unstake(
            poolBump,
            new anchor.BN(5),
            {
                accounts: {
                    xTokenMint: xTokenMintPubkey,
                    yTokenMint: yTokenMintPubkey,
                    withdrawToken: walletYTokenAccount,
                    withdrawTokenAuthority: provider.wallet.publicKey,
                    tokenPool: poolPubkey,
                    receiveToken: walletXTokenAccount,
                    tokenProgram: TOKEN_PROGRAM_ID,
                }
            }
        );
        assert.strictEqual(await utils.getTokenBalance(provider, walletXTokenAccount), 101);
        assert.strictEqual(await utils.getTokenBalance(provider, walletYTokenAccount), 0);
        assert.strictEqual(await utils.getTokenBalance(provider, poolPubkey), 0);
    });

});

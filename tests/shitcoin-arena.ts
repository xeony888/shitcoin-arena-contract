import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ShitcoinArena } from "../target/types/shitcoin_arena";
import { Keypair, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { getAccount, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { assert } from "chai";
import { BN } from "bn.js";

describe("shitcoin-arena", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const wallet = provider.wallet as anchor.Wallet;
  anchor.setProvider(provider);

  const program = anchor.workspace.ShitcoinArena as Program<ShitcoinArena>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().accounts({signer: wallet.publicKey}).rpc();
  });
  it("creates multiple tokens successfully", async () => {
    for (let i = 0; i < 5; i++) {
      const mint = Keypair.generate();
      await program.methods.createTokenAndBuy(new anchor.BN(0)).accounts({
        signer: wallet.publicKey,
        mint: mint.publicKey,
      }).signers([mint]).rpc();
      const [bondingCurveAddress] = PublicKey.findProgramAddressSync(
        [Buffer.from("curve"), mint.publicKey.toBuffer()],
        program.programId,
      );
      const bondingCurve = await program.account.linearBondingCurve.fetch(bondingCurveAddress);
      assert(bondingCurve.closed == false);
      assert(bondingCurve.token.eq(new BN(0)));
    }
  });
  it("creates and buys and sells", async () => {
    const mint = Keypair.generate();
    const tokenAccount = getAssociatedTokenAddressSync(mint.publicKey, wallet.publicKey);
    await program.methods.createTokenAndBuy(new BN(10000)).accounts({
      signer: wallet.publicKey,
      mint: mint.publicKey
    }).signers([mint]).rpc();
    const [bondingCurveAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("curve"), mint.publicKey.toBuffer()],
      program.programId,
    );
    let account = await getAccount(provider.connection, tokenAccount);
    assert(account.amount === BigInt(10000));
    let bondingCurve = await program.account.linearBondingCurve.fetch(bondingCurveAddress);
    assert(bondingCurve.token.eq(new BN(10000)))
    await program.methods.buy(new BN(10000)).accounts({
      signer: wallet.publicKey,
      mint: mint.publicKey,
      signerTokenAccount: tokenAccount,
    }).rpc();
    account = await getAccount(provider.connection, tokenAccount);
    assert(account.amount === BigInt(20000));
    bondingCurve = await program.account.linearBondingCurve.fetch(bondingCurveAddress);
    assert(bondingCurve.token.eq(new BN(20000)));
    await program.methods.sell(new BN(10000)).accounts({
      signer: wallet.publicKey,
      mint: mint.publicKey,
      signerTokenAccount: tokenAccount,
    }).rpc();
    account = await getAccount(provider.connection, tokenAccount);
    assert(account.amount === BigInt(10000));
    bondingCurve = await program.account.linearBondingCurve.fetch(bondingCurveAddress);
    assert(bondingCurve.token.eq(new BN(10000)));
  });
  it("buys until market cap", async () => {
    const mint = Keypair.generate();
    const tokenAccount = getAssociatedTokenAddressSync(mint.publicKey, wallet.publicKey);
    let count = 0;
    program.addEventListener("initializeMigrateEvent", (event) => {
      assert(mint.publicKey.equals(event.mint));
      count++;
    })
    await program.methods.createTokenAndBuy(new BN(2)).accounts({
      signer: wallet.publicKey,
      mint: mint.publicKey,
    }).signers([mint]).rpc();
    const tx =await program.methods.buy(new BN(.5 * 1000000000).mul(new BN(10 ** 6))).accounts({
      signer: wallet.publicKey,
      mint: mint.publicKey,
      signerTokenAccount: tokenAccount
    }).rpc();
    const [bondingCurveAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("curve"), mint.publicKey.toBuffer()],
      program.programId,
    );
    const bondingCurve = await program.account.linearBondingCurve.fetch(bondingCurveAddress);
    assert(bondingCurve.closed, "Bonding curve not closed");
    await new Promise((resolve) => setTimeout(resolve, 1000));
    assert(count === 1, "Did not recieve event");
    console.log("done");
    //assert(bondingCurve.token.eq(new BN(69 * LAMPORTS_PER_SOL).add(new BN(2))), "Wrong location on curve");
  })
  it("buys and sells in single instruction", async () => {
    const from = Keypair.generate()
    const to = Keypair.generate();
    const fromTokenAccount = getAssociatedTokenAddressSync(from.publicKey, wallet.publicKey);
    const toTokenAccount = getAssociatedTokenAddressSync(to.publicKey, wallet.publicKey);
    await program.methods.createTokenAndBuy(new BN(100)).accounts({
      signer: wallet.publicKey,
      mint: from.publicKey
    }).signers([from]).rpc();
    await program.methods.createTokenAndBuy(new BN(100)).accounts({
      signer: wallet.publicKey,
      mint: to.publicKey,
    }).signers([to]).rpc();
    await program.methods.swap(new BN(100), new BN(100)).accounts({
      signer: wallet.publicKey,
      fromMint: from.publicKey,
      toMint: to.publicKey,
      signerFromTokenAccount: fromTokenAccount,
      signerToTokenAccount: toTokenAccount,
    }).rpc();
    const [toBondingCurveAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("curve"), to.publicKey.toBuffer()],
      program.programId,
    );
    const [fromBondingCurveAddress] = PublicKey.findProgramAddressSync(
      [Buffer.from("curve"), from.publicKey.toBuffer()],
      program.programId,
    )
    const toBondingCurve = await program.account.linearBondingCurve.fetch(toBondingCurveAddress);
    const fromBondingCurve = await program.account.linearBondingCurve.fetch(fromBondingCurveAddress);
    assert(toBondingCurve.token.eq(new BN(100).add(new BN(100))), "To bonding curve not correctly changed");
    assert(fromBondingCurve.token.eq(new BN(0)), "From bonding curve not correctly changed");
  });
});

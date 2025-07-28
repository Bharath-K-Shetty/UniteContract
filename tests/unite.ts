import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Unite } from "../target/types/unite";
import {
  Keypair,
  Connection,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

describe("unite", () => {
  const RPC_URL = "http://127.0.0.1:8899";
  const connection = new Connection(RPC_URL, "confirmed");

  const generatedWallet = Keypair.generate();
  const wallet = new anchor.Wallet(generatedWallet);

  const provider = new anchor.AnchorProvider(connection, wallet, {});
  anchor.setProvider(provider);

  const idl = require("../target/idl/unite.json");
  const programId = new PublicKey(idl.metadata.address);
  const program = new anchor.Program<Unite>(idl, programId, provider);

  const authority = provider.wallet.publicKey;

  before("💸 Airdrop SOL to generated wallet", async () => {
    const airdropSig = await connection.requestAirdrop(
      authority,
      LAMPORTS_PER_SOL * 1 // 1 SOL
    );
    await connection.confirmTransaction(airdropSig, "confirmed");

    const balance = await connection.getBalance(authority);
    console.log("✅ Wallet funded with:", balance / LAMPORTS_PER_SOL, "SOL");
  });

  it("initializes the organizer", async () => {
    const [organizerPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("organizer"), authority.toBuffer()],
      program.programId
    );

    try {
      const tx = await program.methods
        .initializeOrganizer()
        .accounts({
          organizer: organizerPda,
          authority,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("✅ Organizer initialized tx:", tx);

      const organizer = await program.account.organizerAccount.fetch(organizerPda);
      console.log("📦 Organizer account:", organizer);
    } catch (e) {
      console.error("❌ Test failed:", e);
      throw e;
    }
  });
});

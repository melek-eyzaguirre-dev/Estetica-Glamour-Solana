import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import type { SalonBelleza } from "../target/types/salon_belleza";
describe("salon_belleza e2e", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SalonBelleza as anchor.Program<SalonBelleza>;
  
  const NOMBRE_SALON = "Estética Glamour Test";
  // Sufijo único para evitar duplicados en la misma PDA al re-ejecutar el test
  const SUFIJO = Math.floor(Date.now() / 1000);
  const SERVICIO_NOMBRE = `Manicure Gel ${SUFIJO}`;
  const SERVICIO_PRECIO_SOL = 0.003;
  const CLIENTE_NOMBRE = `Valentina ${SUFIJO}`;
  const ATENCION_CANTIDAD = 1;

  it("flujo completo", async () => {
    // 0) Wallet y provider
    if (!pg?.wallet?.publicKey) throw new Error("No hay wallet activa en Playground.");
    const ownerPubkey = program.provider.publicKey;

    const provider = new anchor.AnchorProvider(
      program.provider.connection,
      pg.wallet,
      { commitment: "confirmed" }
    );
    anchor.setProvider(provider);

    // 1) Programa
    const program = program;
    if (!program) throw new Error("Compila el programa (Build) antes de ejecutar los tests.");

    // 2) PDA: seeds = ["salon_belleza", owner]
    const [salonPda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("salon_belleza"), ownerPubkey.toBuffer()],
      program.programId
    );

    // 3) crear_salon (crear solo si NO existe)
    {
      const accInfo = await program.provider.connection.getAccountInfo(salonPda);
      if (!accInfo) {
        const tx = await program.methods
          .crearSalon(NOMBRE_SALON)
          .accounts({
            owner: ownerPubkey,
            salon: salonPda,
            systemProgram: web3.SystemProgram.programId,
          })
          .rpc();
        await program.provider.connection.confirmTransaction(tx, "confirmed");
      }
    }

    // 4) agregar_servicio
    const precioLamports = new anchor.BN(
      Math.floor(SERVICIO_PRECIO_SOL * web3.LAMPORTS_PER_SOL)
    );
    const txServicio = await program.methods
      .agregarServicio(SERVICIO_NOMBRE, precioLamports)
      .accounts({ owner: ownerPubkey, salon: salonPda })
      .rpc();
    await program.provider.connection.confirmTransaction(txServicio, "confirmed");

    // 5) registrar_cliente
    const txCliente = await program.methods
      .registrarCliente(CLIENTE_NOMBRE)
      .accounts({ owner: ownerPubkey, salon: salonPda })
      .rpc();
    await program.provider.connection.confirmTransaction(txCliente, "confirmed");

    // 6) registrar_atencion
    const txAtencion = await program.methods
      .registrarAtencion(CLIENTE_NOMBRE, SERVICIO_NOMBRE, ATENCION_CANTIDAD)
      .accounts({ owner: ownerPubkey, salon: salonPda })
      .rpc();
    await program.provider.connection.confirmTransaction(txAtencion, "confirmed");

    // 7) Verificar estado
    const state = await program.account.salonBelleza.fetch(salonPda);
    if (!state) throw new Error("No se pudo leer la cuenta SalonBelleza");

    const servicioEncontrado = state.servicios.find((s) => s.nombre === SERVICIO_NOMBRE);
    if (!servicioEncontrado) throw new Error("No se encontró el servicio registrado");
    if (Number(servicioEncontrado.precio) !== Number(precioLamports)) {
      throw new Error("Precio del servicio no coincide");
    }
    if (servicioEncontrado.realizados !== ATENCION_CANTIDAD) {
      throw new Error(
        `Contador incorrecto: esperado ${ATENCION_CANTIDAD}, obtenido ${servicioEncontrado.realizados}`
      );
    }

    const clienteEncontrado = state.clientes.find((c) => c.nombre === CLIENTE_NOMBRE);
    if (!clienteEncontrado) throw new Error("No se encontró el cliente registrado");
    if (clienteEncontrado.visitas !== 1) {
      throw new Error("Las visitas del cliente no se acumularon");
    }
    if (Number(clienteEncontrado.gastadoTotal) !== Number(precioLamports) * ATENCION_CANTIDAD) {
      throw new Error("gastadoTotal inválido");
    }

    console.log("OK: flujo completo del salón de belleza verificado");
  });
});
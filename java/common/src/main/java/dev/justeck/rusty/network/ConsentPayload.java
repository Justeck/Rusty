package dev.justeck.rusty.network;

import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.resources.Identifier;

/** Server-to-client permission only. No client-to-server grant receiver exists. */
public record ConsentPayload(int mask) implements CustomPacketPayload {
  public static final Type<ConsentPayload> TYPE =
      new Type<>(Identifier.fromNamespaceAndPath("rusty", "permissions_v1"));
  public static final StreamCodec<FriendlyByteBuf, ConsentPayload> CODEC =
      StreamCodec.of(
          (buf, payload) -> {
            buf.writeByte(1);
            buf.writeByte(payload.mask);
          },
          buf -> {
            if (buf.readableBytes() != 2)
              throw new IllegalArgumentException("Invalid Rusty permission payload length");
            int version = buf.readUnsignedByte(), mask = buf.readUnsignedByte();
            if (version != 1 || mask > 15)
              throw new IllegalArgumentException("Unsupported Rusty permissions");
            return new ConsentPayload(mask);
          });

  public ConsentPayload {
    if (mask < 0 || mask > 15) throw new IllegalArgumentException("Invalid permissions");
  }

  @Override
  public Type<? extends CustomPacketPayload> type() {
    return TYPE;
  }
}

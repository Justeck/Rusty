package dev.justeck.rusty;

import dev.justeck.rusty.core.SchematicFence;
import java.util.*;
import net.minecraft.network.Connection;
import net.minecraft.network.protocol.game.ServerboundClientCommandPacket;

/** Ordinary own-player stats reads acknowledge FIFO level-thread processing on vanilla 26.3. */
public final class SchematicStatsBarrier {
  private static final Map<Connection, SchematicFence> CONNECTIONS = new WeakHashMap<>();

  private static synchronized SchematicFence counter(Connection connection) {
    return CONNECTIONS.computeIfAbsent(connection, k -> new SchematicFence());
  }

  public static void sent(Connection connection) {
    counter(connection).sent();
  }

  public static void received(Connection connection) {
    counter(connection).received();
  }

  public static long request() {
    var listener = net.minecraft.client.Minecraft.getInstance().getConnection();
    var counter = counter(listener.getConnection());
    synchronized (counter) {
      listener.send(
          new ServerboundClientCommandPacket(ServerboundClientCommandPacket.Action.REQUEST_STATS));
      return counter.latest();
    }
  }

  public static boolean complete(long token) {
    var listener = net.minecraft.client.Minecraft.getInstance().getConnection();
    return listener != null && counter(listener.getConnection()).complete(token);
  }

  public static boolean tainted() {
    var listener = net.minecraft.client.Minecraft.getInstance().getConnection();
    return listener == null || counter(listener.getConnection()).tainted();
  }
}

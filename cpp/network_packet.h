// This file has been generated from network_packet.oml
#ifndef NETWORK_PACKET_H
#define NETWORK_PACKET_H

#include <cstdint>
#include <string>
#include <optional>
#include <utility>

struct PacketHeader {
public:
	PacketHeader() = default;
	PacketHeader(uint8_t magic_0, uint8_t magic_1, uint8_t magic_2, uint8_t magic_3, uint16_t version, uint32_t sequence, uint16_t payload_len)
		: magic_0(std::move(magic_0))
		, magic_1(std::move(magic_1))
		, magic_2(std::move(magic_2))
		, magic_3(std::move(magic_3))
		, version(std::move(version))
		, sequence(std::move(sequence))
		, payload_len(std::move(payload_len))
	{}

	PacketHeader(const PacketHeader& other) = default;
	PacketHeader(PacketHeader&& other) noexcept = default;
	PacketHeader& operator=(const PacketHeader& other) = default;
	PacketHeader& operator=(PacketHeader&& other) noexcept = default;
	~PacketHeader() = default;

	uint8_t getMagic_0() const { return magic_0; }
	uint8_t getMagic_1() const { return magic_1; }
	uint8_t getMagic_2() const { return magic_2; }
	uint8_t getMagic_3() const { return magic_3; }
	uint16_t getVersion() const { return version; }
	uint32_t getSequence() const { return sequence; }
	uint16_t getPayload_len() const { return payload_len; }

	void setMagic_0(const uint8_t& value) { magic_0 = value; }
	void setMagic_1(const uint8_t& value) { magic_1 = value; }
	void setMagic_2(const uint8_t& value) { magic_2 = value; }
	void setMagic_3(const uint8_t& value) { magic_3 = value; }
	void setVersion(const uint16_t& value) { version = value; }
	void setSequence(const uint32_t& value) { sequence = value; }
	void setPayload_len(const uint16_t& value) { payload_len = value; }
private:
	uint8_t magic_0;
	uint8_t magic_1;
	uint8_t magic_2;
	uint8_t magic_3;
	uint16_t version;
	uint32_t sequence;
	uint16_t payload_len;
};

struct DataPacket {
public:
	DataPacket() = default;
	DataPacket(int32_t header) : header(std::move(header)) {}

	DataPacket(const DataPacket& other) = default;
	DataPacket(DataPacket&& other) noexcept = default;
	DataPacket& operator=(const DataPacket& other) = default;
	DataPacket& operator=(DataPacket&& other) noexcept = default;
	~DataPacket() = default;

	int32_t getHeader() const { return header; }

	void setHeader(const int32_t& value) { header = value; }
private:
	int32_t header;
};

struct DataPacket_payload {
public:
	DataPacket_payload() = default;
	DataPacket_payload(int32_t parent_id, uint8_t value) : parent_id(std::move(parent_id)), value(std::move(value)) {}

	DataPacket_payload(const DataPacket_payload& other) = default;
	DataPacket_payload(DataPacket_payload&& other) noexcept = default;
	DataPacket_payload& operator=(const DataPacket_payload& other) = default;
	DataPacket_payload& operator=(DataPacket_payload&& other) noexcept = default;
	~DataPacket_payload() = default;

	int32_t getParent_id() const { return parent_id; }
	uint8_t getValue() const { return value; }

	void setParent_id(const int32_t& value) { parent_id = value; }
	void setValue(const uint8_t& value) { value = value; }
private:
	int32_t parent_id;
	uint8_t value;
};

struct DataPacket_metadata {
public:
	DataPacket_metadata() = default;
	DataPacket_metadata(int32_t parent_id, std::string value) : parent_id(std::move(parent_id)), value(std::move(value)) {}

	DataPacket_metadata(const DataPacket_metadata& other) = default;
	DataPacket_metadata(DataPacket_metadata&& other) noexcept = default;
	DataPacket_metadata& operator=(const DataPacket_metadata& other) = default;
	DataPacket_metadata& operator=(DataPacket_metadata&& other) noexcept = default;
	~DataPacket_metadata() = default;

	int32_t getParent_id() const { return parent_id; }
	std::string getValue() const { return value; }

	void setParent_id(const int32_t& value) { parent_id = value; }
	void setValue(const std::string& value) { value = value; }
private:
	int32_t parent_id;
	std::string value;
};

enum class PacketType {
	HANDSHAKE'), ('DATA'), ('ACK'), ('DISCONNECT
};
#endif // NETWORK_PACKET_H

